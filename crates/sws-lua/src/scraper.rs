use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{fs, thread};

use crossbeam_channel::{bounded, select, unbounded, Sender};
use mlua::{Function, Lua, LuaSerdeExt};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sws_crawler::{PageLocation, Scrapable, Sitemap};
use sws_scraper::Html;

use crate::interop::{
    LuaElementRef, LuaHtml, LuaPageLocation, LuaSelect, LuaSitemap, LuaStringRecord, LuaSwsContext,
    SwsContext,
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

        // Setup sws namespace

        let sws = lua.create_table()?;

        let select_iter = lua.create_function(move |lua, mut select: LuaSelect| {
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
        sws.set("selectIter", select_iter)?;

        let new_record =
            lua.create_function(|_, ()| Ok(LuaStringRecord(csv::StringRecord::new())))?;
        sws.set("newRecord", new_record)?;

        let location = lua.create_table()?;
        location.set(sws::location::PATH, sws::location::PATH)?;
        location.set(sws::location::URL, sws::location::URL)?;
        sws.set(sws::LOCATION, location)?;

        let sitemap = lua.create_table()?;
        sitemap.set(sws::sitemap::INDEX, sws::sitemap::INDEX)?;
        sitemap.set(sws::sitemap::URL_SET, sws::sitemap::URL_SET)?;
        sws.set(sws::SITEMAP, sitemap)?;

        // Setup globals

        let globals = lua.globals();
        lua.load(&fs::read_to_string(&config.script)?).exec()?;
        globals.set(globals::SWS, sws)?;
        let _: Function = globals.get(globals::ACCEPT_URL)?;
        let _: Function = globals.get(globals::SCRAP_PAGE)?;
        let sitemap_url: String = globals.get(globals::SITEMAP_URL)?;
        let csv_config: writer::CsvWriterConfig = globals
            .get::<_, Option<mlua::Value>>(globals::CSV_WRITER_CONFIG)?
            .map(|h| lua.from_value(h))
            .unwrap_or_else(|| Ok(writer::CsvWriterConfig::default()))?;
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

        let context = LuaSwsContext(Rc::new(SwsContext::new(tx_record.clone())));

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
        let location = LuaPageLocation(location);
        let page = LuaHtml(Html::parse_document(&page));
        let scrap_page: Function = self
            .lua
            .globals()
            .get(globals::SCRAP_PAGE)
            .expect(&format!("Function {} not found", globals::SCRAP_PAGE)); // Ensured in constructor
        Ok(scrap_page
            .call::<_, ()>((page, location, self.context.clone()))
            .map_err(|e| match e {
                mlua::Error::CallbackError { cause, .. } => cause.as_ref().clone(),
                _ => e,
            })?)
    }

    fn accept(&self, sm: Sitemap, url: &str) -> bool {
        let sm = LuaSitemap(sm);
        let accept_url: Function = self
            .lua
            .globals()
            .get(globals::ACCEPT_URL)
            .expect(&format!("Function {} not found", globals::ACCEPT_URL)); // Ensured in constructor

        match accept_url.call::<_, bool>((sm, url.to_string())) {
            Ok(accepted) => accepted,
            Err(e) => {
                match e {
                    mlua::Error::CallbackError { cause, .. } => {
                        log::error!("Couldn't process {sm:?} {url}: {cause}")
                    }
                    _ => log::error!("Couldn't process {sm:?} {url}: {e}"),
                }
                false
            }
        }
    }
}

pub fn scrap_page<P: AsRef<Path>>(
    script: P,
    page: String,
    location: PageLocation,
) -> anyhow::Result<()> {
    let script = script.as_ref().into();

    let temp = tempfile::NamedTempFile::new()?;
    let csv_file = temp.path().into();

    let conf = LuaScraperConfig { script, csv_file };
    let mut scraper = LuaScraper::new(&conf)?;
    scraper.scrap(page, location)?;
    scraper.finalizer();
    drop(scraper);

    let result = fs::read_to_string(temp.path())?;
    println!("{result}");

    Ok(())
}
