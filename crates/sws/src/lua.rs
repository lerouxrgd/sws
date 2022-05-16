use std::path::PathBuf;
use std::rc::Rc;
// use std::sync::mpsc::{channel, Sender};
// use std::sync::Arc;
use std::{fs, thread};

use crossbeam_channel::Sender;
use mlua::{Function, Lua, LuaSerdeExt, MetaMethod, UserData, UserDataMethods};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sws_scraper::{element_ref::Select, ElementRef, Html, Selector};

use crate::crawler::{Scrapable, Sitemap};

static TX_CSV_WRITER: OnceCell<Sender<csv::StringRecord>> = OnceCell::new();

pub struct LuaHtml(Html);

impl UserData for LuaHtml {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("select", |_, html, css_selector: String| {
            let select = html.0.select(Selector::parse(&css_selector).unwrap());
            Ok(LuaSelect(select))
        });
    }
}

#[derive(Clone)]
pub struct LuaSelect(Select);

impl UserData for LuaSelect {}

pub struct LuaElementRef(ElementRef);

impl UserData for LuaElementRef {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("select", |_, elem, css_selector: String| {
            let select = elem.0.select(Selector::parse(&css_selector).unwrap());
            Ok(LuaSelect(select))
        });

        methods.add_method("innerHtml", |_, elem, ()| Ok(elem.0.inner_html()));

        methods.add_method("innerText", |_, elem, ()| Ok(elem.0.inner_text()));
    }
}

pub struct LuaSitemap(Sitemap);

impl UserData for LuaSitemap {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("kind", |_, sm, ()| Ok(format!("{:?}", sm.0)));
    }
}

#[derive(Clone)]
pub struct LuaStringRecord(csv::StringRecord);

impl UserData for LuaStringRecord {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_function(MetaMethod::Call, |_, ()| {
            Ok(LuaStringRecord(csv::StringRecord::new()))
        });

        methods.add_method_mut("pushField", |_, record, field: String| {
            Ok(record.0.push_field(&field))
        });
    }
}

pub struct SwsContext {
    tx_writer: Sender<csv::StringRecord>,
}

impl SwsContext {
    pub fn new(tx_writer: Sender<csv::StringRecord>) -> Self {
        Self { tx_writer }
    }
}

#[derive(Clone)]
pub struct LuaSwsContext(Rc<SwsContext>);

impl UserData for LuaSwsContext {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("sendRecord", |_, ctx, record: LuaStringRecord| {
            ctx.0.tx_writer.send(record.0).ok();
            Ok(())
        });
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CsvWriterConfig {
    #[serde(default = "default_csv_delimiter")]
    delimiter: char,
    #[serde(default)]
    escape: Option<char>,
    #[serde(default)]
    flexible: bool,
    #[serde(default = "default_csv_terminator")]
    terminator: CsvTerminator,
}

impl Default for CsvWriterConfig {
    fn default() -> Self {
        Self {
            delimiter: default_csv_delimiter(),
            escape: None,
            flexible: false,
            terminator: default_csv_terminator(),
        }
    }
}

fn default_csv_delimiter() -> char {
    ','
}

fn default_csv_terminator() -> CsvTerminator {
    CsvTerminator::Any('\n')
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CsvTerminator {
    CRLF,
    Any(char),
}

impl From<CsvTerminator> for csv::Terminator {
    fn from(source: CsvTerminator) -> Self {
        match source {
            CsvTerminator::CRLF => Self::CRLF,
            CsvTerminator::Any(c) => Self::Any(c as u8),
        }
    }
}

impl From<&CsvWriterConfig> for csv::WriterBuilder {
    fn from(c: &CsvWriterConfig) -> Self {
        let mut builder = csv::WriterBuilder::new();
        builder.delimiter(c.delimiter as u8);
        builder.terminator(c.terminator.into());
        builder.flexible(c.flexible);
        if let Some(escape) = c.escape {
            builder.double_quote(false);
            builder.escape(escape as u8);
        } else {
            builder.double_quote(true);
        }
        builder
    }
}

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

        let globals = lua.globals();
        lua.load(&fs::read_to_string(&config.script)?).exec()?;
        globals.set("sws", sws)?;
        let _: Function = globals.get("acceptUrl")?;
        let _: Function = globals.get("processPage")?;
        let sitemap_url: String = globals.get("sitemapUrl")?;
        let csv_config: CsvWriterConfig = globals
            .get::<_, Option<mlua::Value>>("csvWriterConf")?
            .map(|h| lua.from_value(h))
            .unwrap_or_else(|| Ok(CsvWriterConfig::default()))?;
        drop(globals);

        let tx_record = TX_CSV_WRITER.get_or_init(move || {
            let (tx_record, rx_record) = crossbeam_channel::unbounded::<csv::StringRecord>();
            let mut wtr = csv::WriterBuilder::from(&csv_config)
                .from_path(&config.csv_file)
                .unwrap();
            thread::spawn(move || {
                while let Ok(record) = rx_record.recv() {
                    wtr.write_record(record.into_iter()).ok();
                }
            });
            tx_record
        });
        let context = LuaSwsContext(Rc::new(SwsContext::new(tx_record.clone())));

        Ok(Self {
            lua,
            sitemap_url,
            context,
        })
    }

    fn sitemap(&self) -> &str {
        self.sitemap_url.as_ref()
    }

    fn parse(&mut self, page: &str) {
        let page = LuaHtml(Html::parse_document(page));
        let process_page: Function = self
            .lua
            .globals()
            .get("processPage")
            .expect("Function `processPage` not found"); // Ensured in constructor

        // TODO: log instead of unwrap ?
        process_page
            .call::<_, ()>((page, self.context.clone()))
            .unwrap();
    }

    fn accept(&self, sm: Sitemap, url: &str) -> bool {
        let sm = LuaSitemap(sm);
        let accept_url: Function = self
            .lua
            .globals()
            .get("acceptUrl")
            .expect("Function `acceptUrl` not found"); // Ensured in constructor

        // TODO: log instead of unwrap ?
        accept_url.call::<_, bool>((sm, url.to_string())).unwrap()
    }
}
