use std::future::Future;
use std::io::prelude::*;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{cmp, thread};

use anyhow::{anyhow, Error, Result};
use flate2::read::GzDecoder;
use futures::{future, stream, try_join, StreamExt};
use lazy_static::lazy_static;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use sxd_document::{dom, parser};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

lazy_static! {
    static ref HTTP_CLI: reqwest::Client = reqwest::ClientBuilder::new()
        .gzip(true)
        .deflate(true)
        .build()
        .unwrap();
    static ref XP_FACTORY: sxd_xpath::Factory = sxd_xpath::Factory::new();
}

pub trait Scrapable {
    type Config: Clone + Send + 'static;

    fn new(config: &Self::Config) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn sitemap(&self) -> &str;

    fn accept(&self, sm: Sitemap, url: &str) -> bool;

    fn scrap(&mut self, page: &str) -> anyhow::Result<()>;

    fn finalizer(&mut self) {}
}

#[derive(Debug, Clone, Copy)]
pub enum Sitemap {
    Index,
    Urlset,
}

impl Sitemap {
    fn new(root: &dom::Root) -> Self {
        let kind = root.children()[0]
            .element()
            .expect("First child of root is not an element")
            .name()
            .local_part();

        match kind {
            "sitemapindex" => Self::Index,
            "urlset" => Self::Urlset,
            _ => panic!("Unknown root node kind: {}", kind),
        }
    }
}

fn gather_urls<'a, T>(
    config: &'a CrawlerConfig,
    scraper: &'a T,
    sitemap_url: &'a str,
    tx_url: mpsc::UnboundedSender<String>,
) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>>
where
    T: Scrapable,
{
    Box::pin(async move {
        let sitemap_xml = download(config, sitemap_url).await?;

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

        let sm_kind = Sitemap::new(&document.root());

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
                            if let Err(e) = tx_url.send(page_url) {
                                log::error!("Couldn't send page to downloader: {e}");
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    })
}

async fn download(config: &CrawlerConfig, url: &str) -> Result<String> {
    let resp = HTTP_CLI
        .get(url)
        .header(USER_AGENT, &config.user_agent)
        .send()
        .await?;

    match resp.headers().get(CONTENT_TYPE) {
        Some(c) if c == "application/x-gzip" || c == "application/gzip" => {
            let compressed = resp.bytes().await?;
            let mut gz = GzDecoder::new(&compressed[..]);
            let mut page = String::new();
            gz.read_to_string(&mut page)?;
            Ok(page)
        }
        _ => Ok(resp.text().await?),
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlerConfig {
    pub user_agent: String,
    pub page_buffer: usize,
    pub concurrent_downloads: usize,
    pub num_workers: usize,
    pub handle_sigint: bool,
    pub on_dl_error: OnError,
    pub on_xml_error: OnError,
    pub on_scrap_error: OnError,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            user_agent: format!("SWSbot/{}", env!("CARGO_PKG_VERSION")),
            page_buffer: 10_000,
            concurrent_downloads: 100,
            num_workers: cmp::max(1, num_cpus::get().saturating_sub(2)),
            handle_sigint: true,
            on_dl_error: OnError::SkipAndLog,
            on_xml_error: OnError::SkipAndLog,
            on_scrap_error: OnError::SkipAndLog,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ArgEnum))]
pub enum OnError {
    Fail,
    SkipAndLog,
}

pub async fn crawl_site<T>(
    crawler_conf: &CrawlerConfig,
    scraper_conf: &T::Config,
) -> anyhow::Result<()>
where
    T: Scrapable,
{
    let (tx_url, rx_url) = mpsc::unbounded_channel::<String>();
    let (tx_page, rx_page) = crossbeam_channel::bounded::<String>(crawler_conf.page_buffer);

    // Workers

    let stop = Arc::new(AtomicBool::new(false));
    let mut workers = vec![];
    for id in 0..crawler_conf.num_workers {
        let rx_page = rx_page.clone();
        let scraper_conf = scraper_conf.clone();
        let crawler_conf = crawler_conf.clone();
        let stop = stop.clone();
        let worker = thread::Builder::new()
            .name(format!("{id}"))
            .spawn(move || {
                let mut scraper = <T as Scrapable>::new(&scraper_conf)?;
                for page in rx_page.into_iter() {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    match scraper.scrap(&page) {
                        Ok(()) => (),
                        Err(e) => match crawler_conf.on_scrap_error {
                            OnError::SkipAndLog => {
                                log::error!("Skipping page scrap: {e}");
                            }
                            OnError::Fail => {
                                stop.store(true, Ordering::SeqCst);
                                scraper.finalizer();
                                return Err(e);
                            }
                        },
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

    let downloader = async move {
        let stream = UnboundedReceiverStream::new(rx_url)
            .map(|url| async move { download(crawler_conf, &url).await })
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
    let crawler = gather_urls(crawler_conf, &scraper, scraper.sitemap(), tx_url);

    // Run all tasks

    if crawler_conf.handle_sigint {
        let mut scraper = <T as Scrapable>::new(&scraper_conf)?;
        let interrupt = async move {
            tokio::signal::ctrl_c().await?;
            scraper.finalizer();
            Err::<(), _>(anyhow!("Interrupted"))
        };
        try_join!(workers, downloader, crawler, interrupt)?;
    } else {
        try_join!(workers, downloader, crawler)?;
    }

    Ok(())
}
