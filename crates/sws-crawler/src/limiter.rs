use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use futures::stream::{Fuse, FusedStream, FuturesUnordered};
use futures::{future, Future, Stream, StreamExt};
use pin_project_lite::pin_project;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct RateLimiter {
    permits: Arc<Semaphore>,
}

impl RateLimiter {
    pub fn new(per_second: usize) -> Self {
        let permits = Arc::new(Semaphore::new(0));

        let permits_c = permits.clone();
        tokio::spawn(async move {
            loop {
                match timeout(Duration::from_secs(1), future::pending::<()>()).await {
                    Ok(_) => unreachable!(),
                    Err(_) => {
                        let available = permits_c.available_permits();
                        permits_c.add_permits(per_second - available);
                    }
                }
            }
        });

        Self { permits }
    }

    pub fn try_acquire_owned(&self) -> Result<OwnedSemaphorePermit, TryAcquireError> {
        self.permits.clone().try_acquire_owned()
    }
}

pin_project! {
    pub struct PermittedFuture<F> {
        #[pin]
        fut: F,
        permit: Option<OwnedSemaphorePermit>,
    }

    impl<F> PinnedDrop for PermittedFuture<F> {
        fn drop(this: Pin<&mut Self>) {
            let this = this.project();
            if let Some(p) = this.permit.take() { p.forget() }
        }
    }
}

impl<F> Future for PermittedFuture<F>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        this.fut.poll(cx)
    }
}

pin_project! {
    pub struct RateLimited<St>
    where
        St: Stream,
    {
        #[pin]
        stream: Fuse<St>,
        in_progress_queue: FuturesUnordered<PermittedFuture<St::Item>>,
        limiter: RateLimiter,
    }
}

impl<St> fmt::Debug for RateLimited<St>
where
    St: Stream + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RateLimited")
            .field("stream", &self.stream)
            .field("in_progress_queue", &self.in_progress_queue)
            .field("limiter", &self.limiter)
            .finish()
    }
}

impl<St> RateLimited<St>
where
    St: Stream,
    St::Item: Future,
{
    pub fn new(stream: St, limiter: RateLimiter) -> Self
    where
        St: Stream,
        St::Item: Future,
    {
        Self {
            stream: stream.fuse(),
            in_progress_queue: FuturesUnordered::new(),
            limiter,
        }
    }

    // pub fn get_ref(&self) -> &St {
    //     (&self.stream).get_ref()
    // }

    // pub fn get_mut(&mut self) -> &mut St {
    //     (&mut self.stream).get_mut()
    // }

    // pub fn get_pin_mut(self: core::pin::Pin<&mut Self>) -> core::pin::Pin<&mut St> {
    //     self.project().stream.get_pin_mut()
    // }

    // pub fn into_inner(self) -> St {
    //     self.stream.into_inner()
    // }
}

impl<St> Stream for RateLimited<St>
where
    St: Stream,
    St::Item: Future,
{
    type Item = <St::Item as Future>::Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // First up, try to spawn off as many futures as possible
        while let Ok(permit) = this.limiter.try_acquire_owned() {
            match this.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(fut)) => this.in_progress_queue.push(PermittedFuture {
                    permit: Some(permit),
                    fut,
                }),
                Poll::Ready(None) | Poll::Pending => break,
            }
        }

        // Attempt to pull the next value from the in_progress_queue
        match this.in_progress_queue.poll_next_unpin(cx) {
            x @ Poll::Pending | x @ Poll::Ready(Some(_)) => return x,
            Poll::Ready(None) => {}
        }

        // If more values are still coming from the stream, we're not done yet
        if this.stream.is_done() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let queue_len = self.in_progress_queue.len();
        let (lower, upper) = self.stream.size_hint();
        let lower = lower.saturating_add(queue_len);
        let upper = match upper {
            Some(x) => x.checked_add(queue_len),
            None => None,
        };
        (lower, upper)
    }
}

impl<St> FusedStream for RateLimited<St>
where
    St: Stream,
    St::Item: Future,
{
    fn is_terminated(&self) -> bool {
        self.in_progress_queue.is_terminated() && self.stream.is_terminated()
    }
}

pub trait RateLimitedExt: Stream {
    fn rate_limited(self, limiter: RateLimiter) -> RateLimited<Self>
    where
        Self::Item: Future,
        Self: Sized,
    {
        assert_stream::<<Self::Item as Future>::Output, _>(RateLimited::new(self, limiter))
    }
}

impl<T: ?Sized> RateLimitedExt for T where T: Stream {}

fn assert_stream<T, S>(stream: S) -> S
where
    S: Stream<Item = T>,
{
    stream
}
