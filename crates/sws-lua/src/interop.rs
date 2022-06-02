use std::rc::Rc;
use std::{fs, thread};

use crossbeam_channel::Sender;
use mlua::{MetaMethod, UserData, UserDataMethods};
use sws_crawler::{PageLocation, Sitemap};
use sws_scraper::{element_ref::Select, ElementRef, Html, Selector};

use crate::ns::{globals, sws};

pub struct LuaHtml(pub(crate) Html);

impl UserData for LuaHtml {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("select", |_, html, css_selector: String| {
            let select = html.0.select(Selector::parse(&css_selector).map_err(|e| {
                mlua::Error::RuntimeError(format!(
                    "Invalid CSS selector {:?}: {:?}",
                    css_selector, e
                ))
            })?);
            Ok(LuaSelect(select))
        });
    }
}

#[derive(Clone)]
pub struct LuaSelect(pub(crate) Select);

impl UserData for LuaSelect {}

pub struct LuaElementRef(pub(crate) ElementRef);

impl UserData for LuaElementRef {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("select", |_, elem, css_selector: String| {
            let select = elem.0.select(Selector::parse(&css_selector).map_err(|e| {
                mlua::Error::RuntimeError(format!(
                    "Invalid CSS selector {:?}: {:?}",
                    css_selector, e
                ))
            })?);
            Ok(LuaSelect(select))
        });

        methods.add_method("innerHtml", |_, elem, ()| Ok(elem.0.inner_html()));

        methods.add_method("innerText", |_, elem, ()| Ok(elem.0.inner_text()));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LuaSitemap(pub(crate) Sitemap);

impl UserData for LuaSitemap {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("kind", |lua, sm, ()| {
            let sitemap = match sm.0 {
                Sitemap::Index => sws::sitemap::INDEX,
                Sitemap::Urlset => sws::sitemap::URL_SET,
            };
            lua.globals()
                .get::<_, mlua::Table>(globals::SWS)?
                .get::<_, mlua::Table>(sws::SITEMAP)?
                .get::<_, String>(sitemap)
        });
    }
}

#[derive(Clone)]
pub struct LuaStringRecord(pub(crate) csv::StringRecord);

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

#[derive(Debug)]
pub struct LuaPageLocation(pub(crate) PageLocation);

impl UserData for LuaPageLocation {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("kind", |lua, pl, ()| {
            let location = match pl.0 {
                PageLocation::Path(_) => sws::location::PATH,
                PageLocation::Url(_) => sws::location::URL,
            };
            lua.globals()
                .get::<_, mlua::Table>(globals::SWS)?
                .get::<_, mlua::Table>(sws::LOCATION)?
                .get::<_, String>(location)
        });

        methods.add_method("get", |_, pl, ()| {
            Ok(match &pl.0 {
                PageLocation::Path(p) => format!("{}", fs::canonicalize(p)?.display()),
                PageLocation::Url(url) => url.to_string(),
            })
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
pub struct LuaSwsContext(pub(crate) Rc<SwsContext>);

impl UserData for LuaSwsContext {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("sendRecord", |_, ctx, record: LuaStringRecord| {
            ctx.0.tx_writer.send(record.0).ok();
            Ok(())
        });

        methods.add_method("workerId", |_, _, ()| {
            let id = thread::current()
                .name()
                .map(String::from)
                .ok_or_else(|| mlua::Error::RuntimeError("Missing thread name".into()))?;
            Ok(id)
        });
    }
}
