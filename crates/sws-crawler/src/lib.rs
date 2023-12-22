//! Web crawler with plugable scraping logic.
//!
//! The main function [`crawl_site`](crawl_site) crawls and scraps web pages. It is
//! configured through a [`CrawlerConfig`](CrawlerConfig) and a [`Scrapable`](Scrapable)
//! implementation. The latter defines the [`Seed`](Seed) used for crawling, as well as
//! the scraping logic. Note that [robots.txt][robots-txt] seeds are supported and
//! exposed through [texting_robots::Robot][robots] in the
//! [`CrawlingContext`](CrawlingContext) and [`ScrapingContext`](ScrapingContext).
//!
//! [robots-txt]: https://en.wikipedia.org/wiki/Robots.txt
//! [robots]: https://docs.rs/texting_robots/latest/texting_robots/struct.Robot.html

mod config;
mod crawler;
mod limiter;
mod scrapable;

pub use config::{CrawlerConfig, OnError, Throttle};
pub use crawler::crawl_site;
pub use scrapable::{
    CountedTx, CrawlingContext, PageLocation, Scrapable, ScrapingContext, Seed, Sitemap,
};

pub use anyhow;
pub use texting_robots;
