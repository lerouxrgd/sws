pub mod interop;
mod scraper;
pub mod writer;

pub use scraper::{scrap_page, LuaScraper, LuaScraperConfig};
