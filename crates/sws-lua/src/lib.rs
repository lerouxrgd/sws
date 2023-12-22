//! A [sws_crawler::Scrapable][sws_crawler] implementation leveraging [sws_scraper][]
//! CSS selectors and scriptable in Lua.
//!
//! [sws_crawler]: https://crates.io/crates/sws-crawler
//! [sws_scraper]: https://crates.io/crates/sws-scraper

pub mod interop;
pub mod ns;
mod scraper;
pub mod writer;

pub use scraper::{scrap_glob, scrap_page, LuaScraper, LuaScraperConfig};

pub use anyhow;
