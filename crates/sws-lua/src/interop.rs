use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::{fs, thread};

use crossbeam_channel::Sender;
use mlua::{FromLua, MetaMethod, UserData, UserDataMethods};
use sws_crawler::{CountedTx, CrawlingContext, PageLocation, ScrapingContext, Sitemap};
use sws_scraper::CaseSensitivity;
use sws_scraper::{element_ref::Select, ElementRef, Html, Selector};
use texting_robots::Robot;

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

#[derive(Clone, Default)]
pub struct LuaStringRecord(pub(crate) csv::StringRecord);

impl<'lua> FromLua<'lua> for LuaStringRecord {
    fn from_lua(value: mlua::Value<'lua>, _: &'lua mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => unreachable!(),
        }
    }
}

impl UserData for LuaStringRecord {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, r, ()| Ok(format!("{:?}", r.0)));

        methods.add_method_mut(sws::record::PUSH_FIELD, |_, record, field: String| {
            record.0.push_field(&field);
            Ok(())
        });
    }
}

pub struct LuaDate(pub(crate) chrono::NaiveDate);

impl LuaDate {
    pub fn new(d: &str, fmt: &str) -> mlua::Result<Self> {
        Ok(Self(chrono::NaiveDate::parse_from_str(d, fmt).map_err(
            |e| mlua::Error::RuntimeError(format!("Couldn't parse date {d} got: {e}")),
        )?))
    }
}

impl UserData for LuaDate {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, d, ()| Ok(format!("{:?}", d.0)));

        methods.add_method(sws::date::FORMAT, |_, d, fmt: String| {
            Ok(d.0.format(&fmt).to_string())
        });
    }
}

#[derive(Clone, Debug)]
pub struct LuaRobot(pub(crate) Arc<Robot>);

impl UserData for LuaRobot {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, r, ()| Ok(format!("{:?}", r.0)));

        methods.add_method(sws::robot::ALLOWED, |_, r, url: String| {
            Ok(r.0.allowed(&url))
        });
    }
}

#[derive(Clone, Debug)]
pub struct LuaCrawlingContext {
    sm: &'static str,
    robot: Option<LuaRobot>,
}

impl<'lua> FromLua<'lua> for LuaCrawlingContext {
    fn from_lua(value: mlua::Value<'lua>, _: &'lua mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => unreachable!(),
        }
    }
}

impl UserData for LuaCrawlingContext {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, ctx, ()| Ok(format!("{:?}", ctx)));

        methods.add_method(sws::crawling_context::ROBOT, |_, ctx, ()| {
            Ok(ctx.robot.clone())
        });

        methods.add_method(sws::crawling_context::SITEMAP, |_, ctx, ()| Ok(ctx.sm));
    }
}

impl From<CrawlingContext> for LuaCrawlingContext {
    fn from(ctx: CrawlingContext) -> Self {
        Self {
            sm: match ctx.sitemap() {
                Sitemap::Index => sws::sitemap::INDEX,
                Sitemap::Urlset => sws::sitemap::URL_SET,
            },
            robot: ctx.robot().map(LuaRobot),
        }
    }
}

#[derive(Clone)]
pub struct LuaScrapingContext {
    tx_writer: Sender<csv::StringRecord>,
    page_location: Weak<PageLocation>,
    tx_url: Option<CountedTx>,
    robot: Option<Arc<Robot>>,
}

impl LuaScrapingContext {
    pub fn new(tx_writer: Sender<csv::StringRecord>, ctx: ScrapingContext) -> Self {
        Self {
            tx_writer,
            page_location: Rc::downgrade(&ctx.location()),
            tx_url: ctx.tx_url(),
            robot: ctx.robot(),
        }
    }
}

impl UserData for LuaScrapingContext {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method(sws::scraping_context::PAGE_LOCATION, |_, ctx, ()| {
            Ok(LuaPageLocation(ctx.page_location.clone()))
        });

        methods.add_method(
            sws::scraping_context::SEND_RECORD,
            |_, ctx, record: LuaStringRecord| {
                ctx.tx_writer.send(record.0).ok();
                Ok(())
            },
        );

        methods.add_method(sws::scraping_context::WORKER_ID, |_, _, ()| {
            let id = thread::current()
                .name()
                .map(String::from)
                .ok_or_else(|| mlua::Error::RuntimeError("Missing thread name".into()))?;
            Ok(id)
        });

        methods.add_method(sws::scraping_context::SEND_URL, |_, ctx, url: String| {
            if let Some(tx_url) = &ctx.tx_url {
                tx_url.send(url);
            } else {
                log::warn!("Context not initalized, coudln't send URL {url}")
            }
            Ok(())
        });

        methods.add_method(sws::scraping_context::ROBOT, |_, ctx, ()| {
            Ok(ctx.robot.clone().map(LuaRobot))
        });
    }
}
