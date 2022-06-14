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
use futures::{future, stream, try_join, StreamExt};
use lazy_static::lazy_static;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use sxd_document::parser;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::config::{CrawlerConfig, OnError};
use crate::scrapable::{CountedTx, PageLocation, Scrapable, Seed, Sitemap};

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

        let sm_kind = Sitemap::from(document.root());

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
                        .filter(|sm_url| scraper.accept(sm_kind, &sm_url))
                        .map(|url| (url, tx_url.clone()));

                    let stream = stream::iter(urls)
                        .map(|(sm_url, tx_url)| async move {
                            gather_urls(config, scraper, &sm_url, tx_url).await
                        })
                        .buffer_unordered(config.concurrent_downloads);

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
                        if scraper.accept(sm_kind, &page_url) {
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
    let pages_in = Arc::new(AtomicUsize::new(0));
    let pages_out = Arc::new(AtomicUsize::new(0));

    let (tx_stop, rx_stop) = crossbeam_channel::unbounded::<()>();
    let (tx_url, rx_url) = mpsc::unbounded_channel::<String>();
    let (tx_page, rx_page) = crossbeam_channel::bounded::<Page>(crawler_conf.page_buffer);

    let tx_url = CountedTx::new(tx_url, pages_in.clone());

    // Workers

    let stop = Arc::new(AtomicBool::new(false));
    let mut workers = vec![];
    for id in 0..crawler_conf.num_workers {
        let rx_stop = rx_stop.clone();
        let rx_page = rx_page.clone();
        let tx_url = tx_url.clone();
        let pages_out = pages_out.clone();
        let scraper_conf = scraper_conf.clone();
        let crawler_conf = crawler_conf.clone();
        let stop = stop.clone();
        let worker = thread::Builder::new()
            .name(format!("{id}"))
            .spawn(move || {
                let mut scraper = <T as Scrapable>::new(&scraper_conf)?;
                scraper.init(tx_url);
                loop {
                    crossbeam_channel::select! {
                        recv(rx_page) -> page => {
                            if let Ok(Page { page, location }) = page {
                                let location = Rc::new(location);
                                match scraper.scrap(page, location.clone()) {
                                    Ok(()) => (),
                                    Err(e) => match crawler_conf.on_scrap_error {
                                        OnError::SkipAndLog => {
                                            log::error!("Skipping scrap for page {location:?} got: {e}");
                                        }
                                        OnError::Fail => {
                                            stop.store(true, Ordering::SeqCst);
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

    // Downloader

    let pages_in_c = pages_in.clone();
    let downloader = async move {
        let stream = UnboundedReceiverStream::new(rx_url)
            .zip(stream::repeat_with(move || pages_in_c.clone()))
            .map(|(url, pages_in)| async move {
                download(crawler_conf, &url).await.map_err(|e| {
                    pages_in.fetch_sub(1, Ordering::SeqCst);
                    e
                })
            })
            .buffer_unordered(crawler_conf.concurrent_downloads);

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

    // Crawler

    let scraper = <T as Scrapable>::new(&scraper_conf)?;
    let seed = scraper.seed();
    let crawler: Pin<Box<dyn Future<Output = Result<()>>>> = match seed {
        Seed::Sitemaps(urls) => Box::pin(async move {
            for sm_url in urls {
                gather_urls(crawler_conf, &scraper, &sm_url, tx_url.clone()).await?;
            }
            drop(tx_url);
            Ok(())
        }),
        Seed::Pages(urls) => {
            urls.into_iter().for_each(|page_url| {
                tx_url.send(page_url);
            });
            drop(tx_url);
            Box::pin(async move { Ok(()) })
        }
    };

    // Run all tasks

    let done = Box::pin(async move {
        loop {
            match timeout(Duration::from_secs(1), tokio::signal::ctrl_c()).await {
                Ok(_) => return Err::<(), _>(anyhow!("Interrupted")),
                Err(_) => {
                    if pages_out.load(Ordering::SeqCst) == pages_in.load(Ordering::SeqCst) {
                        for _ in 0..crawler_conf.num_workers {
                            tx_stop.send(()).ok();
                        }
                        return Ok::<_, Error>(());
                    }
                }
            }
        }
    });

    let res = try_join!(workers, downloader, crawler, done);
    <T as Scrapable>::new(&scraper_conf)?.finalizer();
    res?;

    Ok(())
}
