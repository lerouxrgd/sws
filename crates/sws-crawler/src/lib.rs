use std::future::Future;
use std::io::prelude::*;
use std::pin::Pin;
use std::{cmp, thread};

use anyhow::{anyhow, Error, Result};
use flate2::read::GzDecoder;
use futures::{future, join, stream, StreamExt};
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
    type Config: Send + Clone + 'static;

    fn new(config: &Self::Config) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn sitemap(&self) -> &str;

    fn accept(&self, sm: Sitemap, url: &str) -> bool;

    fn parse(&mut self, page: &str);
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
    spec: &'a T,
    sitemap_url: &'a str,
    tx_url: mpsc::UnboundedSender<String>,
) -> Pin<Box<dyn 'a + Future<Output = Result<()>>>>
where
    T: Scrapable,
{
    Box::pin(async move {
        let sitemap_xml = download(config, sitemap_url).await?;

        let package = parser::parse(&sitemap_xml)?;
        let document = package.as_document();

        let sm_kind = Sitemap::new(&document.root());

        let mut context = sxd_xpath::Context::new();
        context.set_namespace("sm", "http://www.sitemaps.org/schemas/sitemap/0.9");

        let xpath = XP_FACTORY
            .build("//sm:loc")?
            .ok_or_else(|| anyhow!("Missing XPath"))?;

        let value = xpath.evaluate(&context, document.root())?;
        if let sxd_xpath::Value::Nodeset(nodes) = value {
            match sm_kind {
                Sitemap::Index => {
                    let urls = nodes
                        .iter()
                        .map(|node| node.string_value())
                        .filter(|sm_url| spec.accept(sm_kind, &sm_url))
                        .map(|url| (url, tx_url.clone()));

                    let mut err = Ok::<(), Error>(());

                    stream::iter(urls)
                        .map(|(sm_url, tx_url)| async move {
                            gather_urls(config, spec, &sm_url, tx_url).await
                        })
                        .buffer_unordered(100)
                        .scan(&mut err, until_err)
                        .collect::<Vec<_>>()
                        .await;

                    err?;
                }
                Sitemap::Urlset => {
                    for node in nodes {
                        let page_url = node.string_value();
                        if spec.accept(sm_kind, &page_url) {
                            tx_url.send(page_url)?;
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
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            user_agent: format!("SWSbot/{}", env!("CARGO_PKG_VERSION")),
            page_buffer: 10_000,
            concurrent_downloads: 100,
            num_workers: cmp::max(1, num_cpus::get() - 2),
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
    let (tx_url, rx_url) = mpsc::unbounded_channel::<String>();
    let (tx_page, rx_page) = crossbeam_channel::bounded::<String>(crawler_conf.page_buffer);

    let mut workers = vec![];
    for _ in 0..crawler_conf.num_workers {
        let rx_page_c = rx_page.clone();
        let scraper_conf = scraper_conf.clone();
        let worker = thread::spawn(move || {
            {
                let mut scraper = <T as Scrapable>::new(&scraper_conf)?;
                for page in rx_page_c.into_iter() {
                    scraper.parse(&page); // TODO: could return error
                }
                Ok::<(), Error>(())
            }
            .map_err(|e| e) // TODO: log/process/fail here ?
        });
        workers.push(worker);
    }

    let scraper = <T as Scrapable>::new(&scraper_conf)?;
    let (f1, f2) = join!(
        async move {
            let mut err = Ok::<(), Error>(());

            UnboundedReceiverStream::new(rx_url)
                .map(|url| async move { download(crawler_conf, &url).await })
                .buffer_unordered(crawler_conf.concurrent_downloads)
                .scan(&mut err, until_err)
                .map(|page| tx_page.send(page).ok())
                .collect::<Vec<_>>()
                .await;

            err
        },
        gather_urls(crawler_conf, &scraper, scraper.sitemap(), tx_url)
    );

    f1?;
    f2?;

    for w in workers {
        w.join().unwrap()?;
    }

    Ok(())
}
