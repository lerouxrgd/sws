use std::path::PathBuf;
use std::{fs, thread};

use crossbeam_channel::{bounded, select, unbounded, Sender};
use mlua::{Function, Lua, LuaSerdeExt};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sws_crawler::{CrawlerConfig, OnError, PageLocation, Scrapable, Sitemap};
use sws_scraper::Html;

use crate::interop::{
    LuaElementRef, LuaHtml, LuaSelect, LuaStringRecord, LuaSwsContext, SwsContext,
};
use crate::ns::{globals, sws};
use crate::writer;

static TX_CSV_WRITER: OnceCell<(Sender<csv::StringRecord>, Sender<()>)> = OnceCell::new();

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LuaScraperConfig {
    pub script: PathBuf,
    pub csv_file: PathBuf,
}

pub struct LuaScraper {
    lua: Lua,
    sitemap_url: String,
    context: LuaSwsContext,
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
        let _: Function = globals.get(globals::ACCEPT_URL)?;
        let _: Function = globals.get(globals::SCRAP_PAGE)?;

        // Setup sws namespace

        let sws = globals.get::<_, mlua::Table>(globals::SWS)?;

        let select_iter = lua.create_function(move |lua, mut select: LuaSelect| {
            let iterator =
                lua.create_function_mut(move |_, ()| Ok(select.0.next().map(LuaElementRef)));
            Ok(iterator)
        })?;
        sws.set(sws::ITER, select_iter)?;

        let select_enumerate = lua.create_function(move |lua, mut select: LuaSelect| {
            let mut i = 0;
            let iterator = lua.create_function_mut(move |_, ()| {
                i += 1;
                let next = select.0.next().map(LuaElementRef);
                if next.is_some() {
                    Ok((Some(i), next))
                } else {
                    Ok((None, None))
                }
            });
            Ok(iterator)
        })?;
        sws.set(sws::ENUMERATE, select_enumerate)?;

        let record = lua.create_table()?;
        let new_record =
            lua.create_function(|_, ()| Ok(LuaStringRecord(csv::StringRecord::new())))?;
        record.set(sws::record::NEW, new_record)?;
        sws.set(sws::RECORD, record)?;

        let location = lua.create_table()?;
        location.set(sws::location::PATH, sws::location::PATH)?;
        location.set(sws::location::URL, sws::location::URL)?;
        sws.set(sws::LOCATION, location)?;

        let sitemap = lua.create_table()?;
        sitemap.set(sws::sitemap::INDEX, sws::sitemap::INDEX)?;
        sitemap.set(sws::sitemap::URL_SET, sws::sitemap::URL_SET)?;
        sws.set(sws::SITEMAP, sitemap)?;

        // Retrieve custom values

        let sitemap_url: String = sws.get(sws::SITEMAP_URL).map_err(|e| {
            mlua::Error::RuntimeError(format!(
                "Couldn't read {}.{} got: {}",
                globals::SWS,
                sws::SITEMAP_URL,
                e
            ))
        })?;

        let csv_config: writer::CsvWriterConfig = sws
            .get::<_, Option<mlua::Value>>(sws::CSV_WRITER_CONFIG)?
            .map(|h| lua.from_value(h))
            .unwrap_or_else(|| Ok(writer::CsvWriterConfig::default()))?;

        // Register sws namespace

        globals.set(globals::SWS, sws)?;
        drop(globals);

        // Setup csv writer

        let (tx_record, _) = TX_CSV_WRITER.get_or_try_init::<_, anyhow::Error>(move || {
            let (tx_record, rx_record) = unbounded::<csv::StringRecord>();
            let (tx_stop, rx_stop) = bounded::<()>(1);

            let mut wtr = csv::WriterBuilder::from(&csv_config).from_path(&config.csv_file)?;
            thread::spawn(move || loop {
                select! {
                    recv(rx_stop) -> _ => {
                        wtr.flush().ok();
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

            Ok((tx_record, tx_stop))
        })?;

        // Setup context

        let context = LuaSwsContext::new(SwsContext::new(tx_record.clone()));

        Ok(Self {
            lua,
            sitemap_url,
            context,
        })
    }

    fn finalizer(&mut self) {
        TX_CSV_WRITER.get().map(|(_, tx_stop)| tx_stop.send(()));
    }

    fn sitemap(&self) -> &str {
        self.sitemap_url.as_ref()
    }

    fn scrap(&mut self, page: String, location: PageLocation) -> anyhow::Result<()> {
        let scrap_page: Function = self
            .lua
            .globals()
            .get(globals::SCRAP_PAGE)
            .expect(&format!("Function {} not found", globals::SCRAP_PAGE)); // Ensured in constructor

        let page = LuaHtml(Html::parse_document(&page));
        self.context.borrow_mut().page_location = location;

        Ok(scrap_page
            .call::<_, ()>((page, self.context.clone()))
            .map_err(|e| anyhow::anyhow!(e.to_string().replace('\n', "")))?)
    }

    fn accept(&self, sm: Sitemap, url: &str) -> bool {
        let sm = match sm {
            Sitemap::Index => sws::sitemap::INDEX,
            Sitemap::Urlset => sws::sitemap::URL_SET,
        };

        let accept_url: Function = self
            .lua
            .globals()
            .get(globals::ACCEPT_URL)
            .expect(&format!("Function {} not found", globals::ACCEPT_URL)); // Ensured in constructor

        match accept_url.call::<_, bool>((sm, url.to_string())) {
            Ok(accepted) => accepted,
            Err(e) => {
                log::error!(
                    "Couldn't process {sm:?} {url}: {}",
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

pub fn scrap_dir(
    config: &LuaScraperConfig,
    pattern: &str,
    on_error: OnError,
) -> anyhow::Result<()> {
    let mut scraper = LuaScraper::new(&config)?;
    for path in glob::glob(pattern)? {
        let path = path?;
        match scraper.scrap(fs::read_to_string(&path)?, PageLocation::Path(path)) {
            Ok(()) => (),
            Err(e) => match on_error {
                OnError::SkipAndLog => {
                    log::error!("Skipping page scrap: {e}");
                }
                OnError::Fail => {
                    scraper.finalizer();
                    return Err(e);
                }
            },
        }
    }
    scraper.finalizer();
    Ok(())
}

pub fn scrap_page(
    config: &LuaScraperConfig,
    page: String,
    location: PageLocation,
) -> anyhow::Result<()> {
    let mut scraper = LuaScraper::new(&config)?;
    scraper.scrap(page, location)?;
    scraper.finalizer();
    Ok(())
}
