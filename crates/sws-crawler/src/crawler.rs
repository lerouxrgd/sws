use std::future::Future;
use std::io::prelude::*;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Error, Result};
use flate2::read::GzDecoder;
use futures::{future, stream, try_join, Stream, StreamExt};
use lazy_static::lazy_static;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use sxd_document::parser;
use texting_robots::Robot;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::config::{CrawlerConfig, OnError, Throttle};
use crate::limiter::{RateLimitedExt, RateLimiter};
use crate::scrapable::{
    CountedTx, CrawlingContext, PageLocation, Scrapable, ScrapingContext, Seed, Sitemap,
};

lazy_static! {
    static ref HTTP_CLI: reqwest::Client = reqwest::ClientBuilder::new()
        .gzip(true)
        .deflate(true)
        .build()
        .unwrap();
    static ref XP_FACTORY: sxd_xpath::Factory = sxd_xpath::Factory::new();
}

fn gather_urls<'a, T>(
    config: &'a CrawlerConfig,
    scraper: &'a T,
    sitemap_url: &'a str,
    tx_url: CountedTx,
    throttler: Throttler,
    robot: Option<Arc<Robot>>,
) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>>
where
    T: Scrapable,
{
    Box::pin(async move {
        let Page {
            page: sitemap_xml, ..
        } = download(config, sitemap_url).await?;

        let package = match parser::parse(&sitemap_xml) {
            Ok(package) => package,
            Err(e) => match config.on_xml_error {
                OnError::SkipAndLog => {
                    log::warn!("Skipping XML: {sitemap_url} got: {e}");
                    return Ok(());
                }
                OnError::Fail => return Err(anyhow!("Couldn't parse {sitemap_url} got: {e}")),
            },
        };
        let document = package.as_document();

        let sm_kind = Sitemap::try_from(document.root())?;

        let mut context = sxd_xpath::Context::new();
        context.set_namespace("sm", "http://www.sitemaps.org/schemas/sitemap/0.9");
        let xpath = XP_FACTORY
            .build("//sm:loc")?
            .ok_or_else(|| anyhow!("Missing XPath"))?;
        let value = match xpath.evaluate(&context, document.root()) {
            Ok(value) => value,
            Err(e) => match config.on_xml_error {
                OnError::SkipAndLog => {
                    log::warn!("Skipping XML: {sitemap_url} xpath {xpath:?} got: {e}");
                    return Ok(());
                }
                OnError::Fail => {
                    return Err(anyhow!(
                        "Couldn't evaluate {xpath:?} for {sitemap_url} got: {e}"
                    ))
                }
            },
        };

        if let sxd_xpath::Value::Nodeset(nodes) = value {
            match sm_kind {
                Sitemap::Index => {
                    let urls = nodes
                        .iter()
                        .map(|node| node.string_value())
                        .filter(|sm_url| {
                            let ctx = CrawlingContext::new(sm_kind, robot.clone());
                            scraper.accept(sm_url, ctx)
                        })
                        .map(|url| (url, tx_url.clone(), throttler.clone(), robot.clone()));

                    let stream =
                        stream::iter(urls).map(|(sm_url, tx_url, limiter, robot)| async move {
                            gather_urls(config, scraper, &sm_url, tx_url, limiter, robot).await
                        });
                    let stream = throttler.throttle(stream);

                    match config.on_dl_error {
                        OnError::Fail => {
                            let mut err = Ok::<(), Error>(());
                            stream.scan(&mut err, until_err).collect::<Vec<_>>().await;
                            err?
                        }
                        OnError::SkipAndLog => {
                            stream
                                .filter_map(|dl| async move {
                                    dl.map_err(|e| log::warn!("Skipping URL: {e}")).ok()
                                })
                                .collect::<Vec<_>>()
                                .await;
                        }
                    }
                }
                Sitemap::Urlset => {
                    for node in nodes {
                        let page_url = node.string_value();
                        let ctx = CrawlingContext::new(sm_kind, robot.clone());
                        if scraper.accept(&page_url, ctx) {
                            tx_url.send(page_url);
                        }
                    }
                }
            }
        }

        Ok(())
    })
}

#[derive(Debug, Clone)]
struct Page {
    page: String,
    location: PageLocation,
}

#[derive(Debug, Clone)]
struct Throttler {
    throttle: Throttle,
    limiter: Option<RateLimiter>,
}

impl Throttler {
    pub fn new(throttle: Throttle) -> Self {
        let limiter = match throttle {
            Throttle::Concurrent(_) => None,
            Throttle::PerSecond(n) => Some(RateLimiter::with_limit(n.get())),
            Throttle::Delay(delay) => Some(RateLimiter::with_delay(delay)),
        };
        Self { throttle, limiter }
    }

    pub fn throttle<'a, S, F, T>(
        &self,
        stream: S,
    ) -> Pin<Box<dyn Stream<Item = Result<T, anyhow::Error>> + 'a>>
    where
        S: Stream<Item = F> + 'a,
        F: Future<Output = Result<T, anyhow::Error>>,
    {
        match (self.throttle, &self.limiter) {
            (Throttle::Concurrent(n), _) => stream.buffer_unordered(n.get()).boxed_local(),
            (Throttle::PerSecond(_), Some(limiter)) => {
                stream.rate_limited(limiter.clone()).boxed_local()
            }
            (Throttle::Delay(_), Some(limiter)) => {
                stream.rate_limited(limiter.clone()).boxed_local()
            }
            _ => unreachable!(),
        }
    }
}

async fn download(config: &CrawlerConfig, url: &str) -> Result<Page> {
    let resp = HTTP_CLI
        .get(url)
        .header(USER_AGENT, &config.user_agent)
        .send()
        .await?;

    let page = match resp.headers().get(CONTENT_TYPE) {
        Some(c) if c == "application/x-gzip" || c == "application/gzip" => {
            let compressed = resp.bytes().await?;
            let mut gz = GzDecoder::new(&compressed[..]);
            let mut page = String::new();
            gz.read_to_string(&mut page)?;
            page
        }
        _ => resp.text().await?,
    };

    Ok(Page {
        page,
        location: PageLocation::Url(url.to_string()),
    })
}

fn until_err<T, E>(
    err: &mut &mut Result<(), E>,
    item: Result<T, E>,
) -> impl Future<Output = Option<T>> {
    match item {
        Ok(item) => future::ready(Some(item)),
        Err(e) => {
            **err = Err(e);
            future::ready(None)
        }
    }
}

pub async fn crawl_site<T>(
    crawler_conf: &CrawlerConfig,
    scraper_conf: &T::Config,
) -> anyhow::Result<()>
where
    T: Scrapable,
{
    // Initialize shared components

    let scraper = <T as Scrapable>::new(scraper_conf)?;
    let seed = scraper.seed();

    let (robot, throttle) = match (&seed, &crawler_conf.robot) {
        (Seed::RobotsTxt(_), Some(_)) => anyhow::bail!(
            "Invalid seed config, cannot use Seed::RobotsTxt when `crawler_conf.robot` is defined"
        ),
        (Seed::RobotsTxt(url), None) | (_, Some(url)) => {
            let robot = HTTP_CLI.get(url).send().await?.bytes().await?;
            let robot = Robot::new(&crawler_conf.user_agent, &robot)?;
            let throttle = match (robot.delay, crawler_conf.throttle) {
                (_, Some(throttle)) => throttle,
                (Some(delay), None) => {
                    anyhow::ensure!(delay > 0.0, "Robot delay must be > 0.0");
                    Throttle::Delay(delay)
                }
                (None, None) => Throttle::default(),
            };
            let robot = Some(Arc::new(robot));
            (robot, throttle)
        }
        _ => (None, crawler_conf.throttle.unwrap_or_default()),
    };
    let throttler = Throttler::new(throttle);

    // Setup workers task

    let (tx_stop, rx_stop) = crossbeam_channel::unbounded::<()>();
    let (tx_url, rx_url) = mpsc::unbounded_channel::<String>();
    let (tx_page, rx_page) = crossbeam_channel::bounded::<Page>(crawler_conf.page_buffer);

    let failed = Arc::new(AtomicBool::new(false));
    let pages_in = Arc::new(AtomicUsize::new(0));
    let pages_out = Arc::new(AtomicUsize::new(0));

    let tx_url = CountedTx::new(tx_url, pages_in.clone());

    let mut workers = vec![];
    for id in 0..crawler_conf.num_workers {
        let rx_stop = rx_stop.clone();
        let rx_page = rx_page.clone();
        let tx_url = tx_url.clone();
        let robot = robot.clone();
        let pages_out = pages_out.clone();
        let scraper_conf = scraper_conf.clone();
        let crawler_conf = crawler_conf.clone();
        let failed = failed.clone();
        let worker = thread::Builder::new()
            .name(format!("{id}"))
            .spawn(move || {
                let mut scraper = <T as Scrapable>::new(&scraper_conf)?;
                loop {
                    crossbeam_channel::select! {
                        recv(rx_page) -> page => {
                            if failed.load(Ordering::Relaxed) {
                                break;
                            }
                            if let Ok(Page { page, location }) = page {
                                let location = Rc::new(location);
                                let ctx = ScrapingContext::new (
                                    location.clone(),
                                    Some(tx_url.clone()),
                                    robot.clone()
                                );
                                match scraper.scrap(page, ctx) {
                                    Ok(()) => (),
                                    Err(e) => match crawler_conf.on_scrap_error {
                                        OnError::SkipAndLog => {
                                            log::error!("Skipping scrap for page {location:?} got: {e}");
                                        }
                                        OnError::Fail => {
                                            failed.store(true, Ordering::SeqCst);
                                            return Err(e);
                                        }
                                    },
                                }
                                pages_out.fetch_add(1, Ordering::SeqCst);
                            } else {
                                break
                            }
                        },
                        recv(rx_stop) -> _ => break
                    }
                }
                Ok::<(), Error>(())
            })?;
        workers.push(worker);
    }
    let workers = async move {
        tokio::task::spawn_blocking(|| {
            for w in workers {
                w.join().unwrap()?;
            }
            Ok::<(), Error>(())
        })
        .await?
    };

    // Setup crawler task

    let throttler_c = throttler.clone();

    let crawler_done = Arc::new(AtomicBool::new(false));
    let crawler_done_c = crawler_done.clone();

    let crawler: Pin<Box<dyn Future<Output = Result<()>>>> = match seed {
        Seed::Sitemaps(urls) => Box::pin(async move {
            for sm_url in urls {
                gather_urls(
                    crawler_conf,
                    &scraper,
                    &sm_url,
                    tx_url.clone(),
                    throttler_c.clone(),
                    robot.clone(),
                )
                .await?;
            }
            crawler_done_c.store(true, Ordering::SeqCst);
            drop(tx_url);
            Ok(())
        }),
        Seed::RobotsTxt(_) => Box::pin(async move {
            if let Some(r) = &robot {
                let crawling_ctx = CrawlingContext::new(Sitemap::Index, robot.clone());
                for sm_url in &r.sitemaps {
                    if scraper.accept(sm_url, crawling_ctx.clone()) {
                        gather_urls(
                            crawler_conf,
                            &scraper,
                            sm_url,
                            tx_url.clone(),
                            throttler_c.clone(),
                            robot.clone(),
                        )
                        .await?;
                    }
                }
            }
            Ok(())
        }),
        Seed::Pages(urls) => {
            urls.into_iter().for_each(|page_url| {
                tx_url.send(page_url);
            });
            crawler_done_c.store(true, Ordering::SeqCst);
            drop(tx_url);
            Box::pin(async move { Ok(()) })
        }
    };

    // Setup downloader task

    let pages_in_c = pages_in.clone();

    let downloader = async move {
        let stream = UnboundedReceiverStream::new(rx_url)
            .zip(stream::repeat_with(move || pages_in_c.clone()))
            .map(|(url, pages_in)| async move {
                download(crawler_conf, &url).await.map_err(|e| {
                    pages_in.fetch_sub(1, Ordering::SeqCst);
                    e
                })
            });
        let stream = throttler.throttle(stream);

        match crawler_conf.on_dl_error {
            OnError::Fail => {
                let mut err = Ok::<(), Error>(());
                stream
                    .scan(&mut err, until_err)
                    .map(|page| tx_page.send(page).ok())
                    .collect::<Vec<_>>()
                    .await;
                err
            }
            OnError::SkipAndLog => {
                stream
                    .filter_map(
                        |dl| async move { dl.map_err(|e| log::warn!("Skipping URL: {e}")).ok() },
                    )
                    .map(|page| tx_page.send(page).ok())
                    .collect::<Vec<_>>()
                    .await;

                Ok(())
            }
        }
    };

    // Run all tasks

    let done = Box::pin(async move {
        loop {
            match timeout(Duration::from_secs(1), tokio::signal::ctrl_c()).await {
                Ok(_) => return Err(anyhow!("Interrupted")),
                Err(_) => {
                    if pages_out.load(Ordering::SeqCst) == pages_in.load(Ordering::SeqCst)
                        && crawler_done.load(Ordering::SeqCst)
                    {
                        for _ in 0..crawler_conf.num_workers {
                            tx_stop.send(()).ok();
                        }
                        return Ok(());
                    }
                }
            }
        }
    });

    let mut scraper = <T as Scrapable>::new(scraper_conf)?;
    let res = try_join!(workers, downloader, crawler, done);
    scraper.finalizer();
    res?;

    Ok(())
}
