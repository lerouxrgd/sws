use std::path::PathBuf;
use std::{fs, thread};

use crossbeam_channel::{bounded, select, unbounded, Receiver, Sender};
use mlua::{Function, Lua, LuaSerdeExt};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sws_crawler::{
    CrawlerConfig, CrawlingContext, OnError, PageLocation, Scrapable, ScrapingContext, Seed,
};
use sws_scraper::Html;

use crate::interop::{LuaCrawlingContext, LuaDate, LuaHtml, LuaScrapingContext, LuaStringRecord};
use crate::ns::{globals, sws};
use crate::writer;

static TX_CSV_WRITER: OnceCell<(Sender<csv::StringRecord>, Sender<()>, Receiver<()>)> =
    OnceCell::new();

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LuaScraperConfig {
    pub script: PathBuf,
    pub csv_file: Option<PathBuf>,
    pub file_mode: Option<writer::FileMode>,
}

pub struct LuaScraper {
    lua: Lua,
    seed: Seed,
    tx_record: Sender<csv::StringRecord>,
}

impl Scrapable for LuaScraper {
    type Config = LuaScraperConfig;

    fn new(config: &LuaScraperConfig) -> anyhow::Result<Self> {
        let lua = Lua::new();
        let globals = lua.globals();

        // Load and check script

        let sws = lua.create_table()?;
        globals.set(globals::SWS, sws)?;
        lua.load(&fs::read_to_string(&config.script)?).exec()?;
        let _: Function = globals.get(globals::SCRAP_PAGE)?;

        if globals
            .get::<_, Option<Function>>(globals::ACCEPT_URL)?
            .is_none()
        {
            let accept_url =
                lua.create_function(|_, (_url, _ctx): (String, LuaCrawlingContext)| Ok(true))?;
            globals.set(globals::ACCEPT_URL, accept_url)?;
        }

        // Setup sws namespace

        let sws = globals.get::<_, mlua::Table>(globals::SWS)?;

        let new_record = lua.create_function(|_, ()| Ok(LuaStringRecord::default()))?;
        sws.set(sws::RECORD, new_record)?;

        let new_date =
            lua.create_function(|_, (d, fmt): (String, String)| LuaDate::new(&d, &fmt))?;
        sws.set(sws::DATE, new_date)?;

        let location = lua.create_table()?;
        location.set(sws::location::PATH, sws::location::PATH)?;
        location.set(sws::location::URL, sws::location::URL)?;
        sws.set(sws::LOCATION, location)?;

        let sitemap = lua.create_table()?;
        sitemap.set(sws::sitemap::INDEX, sws::sitemap::INDEX)?;
        sitemap.set(sws::sitemap::URL_SET, sws::sitemap::URL_SET)?;
        sws.set(sws::SITEMAP, sitemap)?;

        // Retrieve custom values

        let sitemap_urls: Option<Vec<String>> = sws.get(sws::SEED_SITEMAPS).map_err(|e| {
            mlua::Error::RuntimeError(format!(
                "Couldn't read {}.{} got: {}",
                globals::SWS,
                sws::SEED_SITEMAPS,
                e
            ))
        })?;

        let seed_urls: Option<Vec<String>> = sws.get(sws::SEED_PAGES).map_err(|e| {
            mlua::Error::RuntimeError(format!(
                "Couldn't read {}.{} got: {}",
                globals::SWS,
                sws::SEED_PAGES,
                e
            ))
        })?;

        let seed_robots: Option<String> = sws.get(sws::SEED_ROBOTS_TXT).map_err(|e| {
            mlua::Error::RuntimeError(format!(
                "Couldn't read {}.{} got: {}",
                globals::SWS,
                sws::SEED_ROBOTS_TXT,
                e
            ))
        })?;

        let seed = match (sitemap_urls, seed_urls, seed_robots) {
            (Some(urls), None, None) => Seed::Sitemaps(urls),
            (None, Some(urls), None) => Seed::Pages(urls),
            (None, None, Some(url)) => Seed::RobotsTxt(url),
            _ => anyhow::bail!(
                "Invalid seed, requires exactly one of: {ns}.{s1}, {ns}.{s2}, {ns}.{s3}",
                ns = globals::SWS,
                s1 = sws::SEED_SITEMAPS,
                s2 = sws::SEED_PAGES,
                s3 = sws::SEED_ROBOTS_TXT
            ),
        };

        let csv_config: writer::CsvWriterConfig = sws
            .get::<_, Option<mlua::Value>>(sws::CSV_WRITER_CONFIG)?
            .map(|h| lua.from_value(h))
            .unwrap_or_else(|| Ok(writer::CsvWriterConfig::default()))?;

        // Register sws namespace

        globals.set(globals::SWS, sws)?;
        drop(globals);

        // Setup csv writer

        let (tx_record, _, _) = TX_CSV_WRITER.get_or_try_init::<_, anyhow::Error>(move || {
            let (tx_record, rx_record) = unbounded::<csv::StringRecord>();
            let (tx_stop, rx_stop) = bounded::<()>(1);
            let (tx_done, rx_done) = bounded::<()>(1);

            let mut wtr = match &config.csv_file {
                Some(path) => {
                    let opts: fs::OpenOptions = config
                        .file_mode
                        .as_ref()
                        .map(Clone::clone)
                        .unwrap_or_default()
                        .into();
                    let wtr = csv::WriterBuilder::from(&csv_config).from_writer(opts.open(path)?);
                    writer::CsvWriter::File(wtr)
                }
                None => {
                    let wtr = csv::WriterBuilder::from(&csv_config).from_writer(std::io::stdout());
                    writer::CsvWriter::Stdout(wtr)
                }
            };

            thread::spawn(move || loop {
                select! {
                    recv(rx_stop) -> _ => {
                        wtr.flush().ok();
                        tx_done.send(()).ok();
                        break;
                    },
                    recv(rx_record) -> msg => {
                        msg.map(|record| wtr.write_record(record.into_iter()))
                            .map(|res| if let Err(e) = res {
                                log::error!("Couldn't write record: {e}");
                            })
                            .ok();
                    }
                }
            });

            Ok((tx_record, tx_stop, rx_done))
        })?;

        // Setup context

        Ok(Self {
            lua,
            seed,
            tx_record: tx_record.clone(),
        })
    }

    fn finalizer(&mut self) {
        TX_CSV_WRITER.get().map(|(_, tx_stop, rx_done)| {
            tx_stop.send(()).ok();
            rx_done.recv().ok()
        });
    }

    fn seed(&self) -> Seed {
        self.seed.clone()
    }

    fn scrap(&mut self, page: String, scraping_context: ScrapingContext) -> anyhow::Result<()> {
        let scrap_page: Function = self
            .lua
            .globals()
            .get(globals::SCRAP_PAGE)
            .unwrap_or_else(|_| panic!("Function {} not found", globals::SCRAP_PAGE)); // Ensured in constructor

        let page = LuaHtml(Html::parse_document(&page));
        let ctx = LuaScrapingContext::new(self.tx_record.clone(), scraping_context);

        scrap_page
            .call::<_, ()>((page, ctx))
            .map_err(|e| anyhow::anyhow!(e.to_string().replace('\n', "")))
    }

    fn accept(&self, url: &str, crawling_ctx: CrawlingContext) -> bool {
        let accept_url: Function = self
            .lua
            .globals()
            .get(globals::ACCEPT_URL)
            .unwrap_or_else(|_| panic!("Function {} not found", globals::ACCEPT_URL)); // Ensured in constructor

        let ctx: LuaCrawlingContext = crawling_ctx.clone().into();
        match accept_url.call::<_, bool>((url.to_string(), ctx)) {
            Ok(accepted) => accepted,
            Err(e) => {
                log::error!(
                    "Couldn't process URL {url} ({crawling_ctx:?}) in function {}: {}",
                    globals::ACCEPT_URL,
                    e.to_string().replace('\n', "")
                );
                false
            }
        }
    }
}

impl TryFrom<&LuaScraperConfig> for CrawlerConfig {
    type Error = anyhow::Error;

    fn try_from(c: &LuaScraperConfig) -> Result<Self, Self::Error> {
        let lua = Lua::new();
        let globals = lua.globals();

        let sws = lua.create_table()?;
        globals.set(globals::SWS, sws)?;
        lua.load(&fs::read_to_string(&c.script)?).exec()?;

        let crawler_config: CrawlerConfig = globals
            .get::<_, mlua::Table>(globals::SWS)?
            .get::<_, Option<mlua::Value>>(sws::CRAWLER_CONFIG)?
            .map(|h| lua.from_value(h))
            .unwrap_or_else(|| Ok(CrawlerConfig::default()))?;

        Ok(crawler_config)
    }
}

pub fn scrap_glob(
    config: &LuaScraperConfig,
    pattern: &str,
    on_error: OnError,
    num_workers: usize,
) -> anyhow::Result<()> {
    let (tx_path, rx_path) = unbounded::<PathBuf>();

    let mut workers = vec![];
    for id in 0..num_workers {
        let rx_path = rx_path.clone();
        let config = config.clone();
        let worker = thread::Builder::new()
            .name(format!("{id}"))
            .spawn(move || {
                let mut scraper = LuaScraper::new(&config)?;
                for path in rx_path.into_iter() {
                    let page = fs::read_to_string(&path)?;
                    let ctx = ScrapingContext::with_location(PageLocation::Path(path));
                    match scraper.scrap(page, ctx) {
                        Ok(()) => (),
                        Err(e) => match on_error {
                            OnError::SkipAndLog => {
                                log::error!("Skipping page scrap: {e}");
                            }
                            OnError::Fail => {
                                return Err(e);
                            }
                        },
                    }
                }
                Ok::<(), anyhow::Error>(())
            })?;
        workers.push(worker);
    }

    for path in glob::glob(pattern)? {
        tx_path.send(path?).ok();
    }
    drop(tx_path);

    for w in workers {
        w.join().unwrap()?;
    }

    Ok(())
}

pub fn scrap_page(
    config: &LuaScraperConfig,
    page: String,
    location: PageLocation,
) -> anyhow::Result<()> {
    let mut scraper = LuaScraper::new(config)?;
    scraper.scrap(page, ScrapingContext::with_location(location))?;
    scraper.finalizer();
    Ok(())
}
