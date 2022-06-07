use std::cell::RefCell;
use std::cell::{Ref, RefMut};
use std::path::PathBuf;
use std::rc::Rc;
use std::{fs, thread};

use crossbeam_channel::Sender;
use mlua::{MetaMethod, UserData, UserDataMethods};
use sws_crawler::PageLocation;
use sws_scraper::{element_ref::Select, ElementRef, Html, Selector};

use crate::ns::{globals, sws};

pub struct LuaHtml(pub(crate) Html);
impl UserData for LuaHtml {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method(sws::htlm::SELECT, |_, html, css_selector: String| {
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
        methods.add_meta_method(MetaMethod::ToString, |_, elem, ()| {
            Ok(format!("{:?}", elem.0))
        });

        methods.add_method(sws::elem_ref::SELECT, |_, elem, css_selector: String| {
            let select = elem.0.select(Selector::parse(&css_selector).map_err(|e| {
                mlua::Error::RuntimeError(format!(
                    "Invalid CSS selector {:?}: {:?}",
                    css_selector, e
                ))
            })?);
            Ok(LuaSelect(select))
        });

        methods.add_method(sws::elem_ref::INNER_HTML, |_, elem, ()| {
            Ok(elem.0.inner_html())
        });

        methods.add_method(sws::elem_ref::INNER_TEXT, |_, elem, ()| {
            Ok(elem.0.inner_text())
        });
    }
}

#[derive(Clone)]
pub struct LuaStringRecord(pub(crate) csv::StringRecord);
impl UserData for LuaStringRecord {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, r, ()| Ok(format!("{:?}", r.0)));

        methods.add_method_mut(sws::record::PUSH_FIELD, |_, record, field: String| {
            Ok(record.0.push_field(&field))
        });
    }
}

#[derive(Debug)]
pub struct LuaPageLocation(pub(crate) PageLocation);
impl UserData for LuaPageLocation {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, pl, ()| Ok(format!("{:?}", pl.0)));

        methods.add_method(sws::page_location::KIND, |lua, pl, ()| {
            let location = match pl.0 {
                PageLocation::Path(_) => sws::location::PATH,
                PageLocation::Url(_) => sws::location::URL,
            };
            lua.globals()
                .get::<_, mlua::Table>(globals::SWS)?
                .get::<_, mlua::Table>(sws::LOCATION)?
                .get::<_, String>(location)
        });

        methods.add_method(sws::page_location::GET, |_, pl, ()| {
            Ok(match &pl.0 {
                PageLocation::Path(p) => format!("{}", fs::canonicalize(p)?.display()),
                PageLocation::Url(url) => url.to_string(),
            })
        });
    }
}

pub struct SwsContext {
    pub(crate) tx_writer: Sender<csv::StringRecord>,
    pub(crate) page_location: PageLocation,
}

impl SwsContext {
    pub fn new(tx_writer: Sender<csv::StringRecord>) -> Self {
        // Dummy value, real values will be set by the scrapper for every page
        let page_location = PageLocation::Path(PathBuf::from("/dev/null"));

        Self {
            tx_writer,
            page_location,
        }
    }
}

#[derive(Clone)]
pub struct LuaSwsContext(Rc<RefCell<SwsContext>>);

impl LuaSwsContext {
    pub fn new(ctx: SwsContext) -> Self {
        Self(Rc::new(RefCell::new(ctx)))
    }

    pub fn borrow(&self) -> Ref<'_, SwsContext> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, SwsContext> {
        self.0.borrow_mut()
    }
}

impl UserData for LuaSwsContext {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method(sws::context::PAGE_LOCATION, |_, ctx, ()| {
            Ok(LuaPageLocation(ctx.borrow().page_location.clone()))
        });

        methods.add_method(
            sws::context::SEND_RECORD,
            |_, ctx, record: LuaStringRecord| {
                ctx.borrow().tx_writer.send(record.0).ok();
                Ok(())
            },
        );

        methods.add_method(sws::context::WORKER_ID, |_, _, ()| {
            let id = thread::current()
                .name()
                .map(String::from)
                .ok_or_else(|| mlua::Error::RuntimeError("Missing thread name".into()))?;
            Ok(id)
        });
    }
}
