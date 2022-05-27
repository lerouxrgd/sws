use std::rc::Rc;
use std::thread;

use crossbeam_channel::Sender;
use mlua::{MetaMethod, UserData, UserDataMethods};
use sws_crawler::Sitemap;
use sws_scraper::{element_ref::Select, ElementRef, Html, Selector};

pub struct LuaHtml(pub(crate) Html);

impl UserData for LuaHtml {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("select", |_, html, css_selector: String| {
            let select = html.0.select(Selector::parse(&css_selector).unwrap());
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
            let select = elem.0.select(Selector::parse(&css_selector).unwrap());
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
        methods.add_method("kind", |_, sm, ()| Ok(format!("{:?}", sm.0)));
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
