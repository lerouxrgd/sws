use std::cell::RefCell;
use std::cell::{Ref, RefMut};
use std::rc::{Rc, Weak};
use std::{fs, thread};

use crossbeam_channel::Sender;
use mlua::{MetaMethod, UserData, UserDataMethods};
use sws_crawler::{CountedTx, PageLocation};
use sws_scraper::CaseSensitivity;
use sws_scraper::{element_ref::Select, ElementRef, Html, Selector};

use crate::ns::{globals, sws};

pub struct LuaHtml(pub(crate) Html);
impl UserData for LuaHtml {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, html, ()| {
            Ok(format!("{:?}", html.0))
        });

        methods.add_method(sws::html::SELECT, |_, html, css_selector: String| {
            let select = html.0.select(Selector::parse(&css_selector).map_err(|e| {
                mlua::Error::RuntimeError(format!(
                    "Invalid CSS selector {:?}: {:?}",
                    css_selector, e
                ))
            })?);
            Ok(LuaSelect(select))
        });

        methods.add_method(sws::html::ROOT, |_, html, ()| {
            Ok(LuaElementRef(html.0.root_element()))
        });
    }
}

#[derive(Clone)]
pub struct LuaSelect(pub(crate) Select);
impl UserData for LuaSelect {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, sel, ()| {
            Ok(format!("{:?}", sel.0))
        });

        methods.add_method(sws::select::ITER, |lua, select, ()| {
            let mut select = select.clone();
            let iterator =
                lua.create_function_mut(move |_, ()| Ok(select.0.next().map(LuaElementRef)));

            Ok(iterator)
        });

        methods.add_method(sws::select::ENUMERATE, |lua, select, ()| {
            let mut select = select.clone();
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
        });
    }
}

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

        methods.add_method(sws::elem_ref::NAME, |_, elem, ()| {
            Ok(elem.0.map_value(|el| el.name().to_string()))
        });

        methods.add_method(sws::elem_ref::ID, |_, elem, ()| {
            Ok(elem
                .0
                .map_value(|el| el.id().map(String::from))
                .unwrap_or(None))
        });

        methods.add_method(sws::elem_ref::HAS_CLASS, |_, elem, class: String| {
            Ok(elem
                .0
                .map_value(|el| el.has_class(&class, CaseSensitivity::AsciiCaseInsensitive)))
        });

        methods.add_method(sws::elem_ref::CLASSES, |lua, elem, ()| {
            let classes = lua.create_table()?;
            elem.0.map_value(|el| {
                el.classes().enumerate().for_each(|(i, c)| {
                    classes.set(i + 1, c).ok();
                });
            });
            Ok(classes)
        });

        methods.add_method(sws::elem_ref::ATTR, |_, elem, attr: String| {
            Ok(elem
                .0
                .map_value(|el| el.attr(&attr).map(String::from))
                .unwrap_or(None))
        });

        methods.add_method(sws::elem_ref::ATTRS, |lua, elem, ()| {
            let attrs = lua.create_table()?;
            elem.0.map_value(|el| {
                el.attrs().for_each(|(k, v)| {
                    attrs.set(k, v).ok();
                });
            });
            Ok(attrs)
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
pub struct LuaPageLocation(pub(crate) Weak<PageLocation>);
impl UserData for LuaPageLocation {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, pl, ()| Ok(format!("{:?}", pl.0)));

        methods.add_method(sws::page_location::KIND, |lua, pl, ()| {
            if let Some(loc) = pl.0.upgrade() {
                let location = match loc.as_ref() {
                    PageLocation::Path(_) => sws::location::PATH,
                    PageLocation::Url(_) => sws::location::URL,
                };
                lua.globals()
                    .get::<_, mlua::Table>(globals::SWS)?
                    .get::<_, mlua::Table>(sws::LOCATION)?
                    .get::<_, String>(location)
                    .map(Some)
            } else {
                Ok(None)
            }
        });

        methods.add_method(sws::page_location::GET, |_, pl, ()| {
            if let Some(loc) = pl.0.upgrade() {
                let loc = match loc.as_ref() {
                    PageLocation::Path(p) => format!("{}", fs::canonicalize(p)?.display()),
                    PageLocation::Url(url) => url.to_string(),
                };
                Ok(Some(loc))
            } else {
                Ok(None)
            }
        });
    }
}

pub struct SwsContext {
    pub(crate) tx_writer: Sender<csv::StringRecord>,
    pub(crate) page_location: Weak<PageLocation>,
    pub(crate) tx_url: Option<CountedTx>,
}

impl SwsContext {
    pub fn new(tx_writer: Sender<csv::StringRecord>) -> Self {
        Self {
            tx_writer,
            page_location: Weak::new(),
            tx_url: None,
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

        methods.add_method(sws::context::SEND_URL, |_, ctx, url: String| {
            if let Some(tx_url) = &ctx.borrow().tx_url {
                tx_url.send(url);
            } else {
                log::warn!("Context not initalized, coudln't send URL {url}")
            }
            Ok(())
        });
    }
}
