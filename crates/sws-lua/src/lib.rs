pub mod interop;
pub mod ns;
mod scraper;
pub mod writer;

pub use scraper::{scrap_page, LuaScraper, LuaScraperConfig};
