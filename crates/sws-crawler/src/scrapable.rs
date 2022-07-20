use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, bail};
use sxd_document::dom;
use texting_robots::Robot;
use tokio::sync::mpsc;

pub trait Scrapable {
    type Config: Clone + Send + 'static;

    fn new(config: &Self::Config) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn init(&mut self, _tx_url: CountedTx, _robot: Option<Arc<Robot>>) {}

    fn seed(&self) -> Seed;

    fn accept(&self, url: &str, crawling_ctx: CrawlingContext) -> bool;

    fn scrap(&mut self, page: String, location: Rc<PageLocation>) -> anyhow::Result<()>;

    fn finalizer(&mut self) {}
}

#[derive(Debug, Clone)]
pub enum Seed {
    Sitemaps(Vec<String>),
    Pages(Vec<String>),
    RobotsTxt(String),
}

#[derive(Debug, Clone)]
pub struct CrawlingContext {
    sitemap: Sitemap,
    robot: Option<Arc<Robot>>,
}

impl CrawlingContext {
    pub(crate) fn new(sm: Sitemap, robot: Option<Arc<Robot>>) -> Self {
        Self { sitemap: sm, robot }
    }

    pub fn sitemap(&self) -> Sitemap {
        self.sitemap
    }

    pub fn robot(&self) -> Option<Arc<Robot>> {
        self.robot.clone()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Sitemap {
    Index,
    Urlset,
}

impl<'a> TryFrom<dom::Root<'a>> for Sitemap {
    type Error = anyhow::Error;

    fn try_from(root: dom::Root<'a>) -> Result<Self, Self::Error> {
        let kind = root.children()[0]
            .element()
            .ok_or_else(|| anyhow!("First child of root is not an element"))?
            .name()
            .local_part();

        let sm = match kind {
            "sitemapindex" => Self::Index,
            "urlset" => Self::Urlset,
            _ => bail!("Unknown root node kind: {}", kind),
        };

        Ok(sm)
    }
}

#[derive(Debug, Clone)]
pub enum PageLocation {
    Url(String),
    Path(PathBuf),
}

#[derive(Debug, Clone)]
pub struct CountedTx {
    tx: mpsc::UnboundedSender<String>,
    counter: Arc<AtomicUsize>,
}

impl CountedTx {
    pub fn new(tx: mpsc::UnboundedSender<String>, counter: Arc<AtomicUsize>) -> Self {
        Self { tx, counter }
    }

    pub fn send(&self, s: String) {
        match self.tx.send(s) {
            Ok(()) => {
                self.counter.fetch_add(1, Ordering::SeqCst);
            }
            Err(e) => {
                log::error!("Couldn't send data: {e}");
            }
        }
    }
}
