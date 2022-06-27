use std::cmp;
use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlerConfig {
    #[serde(default = "default_user_agent")]
    pub user_agent: String,

    #[serde(default = "default_page_buffer")]
    pub page_buffer: usize,

    #[serde(default = "default_throttle")]
    pub throttle: Option<Throttle>,

    #[serde(default = "default_num_workers")]
    pub num_workers: usize,

    #[serde(default = "default_on_dl_error")]
    pub on_dl_error: OnError,

    #[serde(default = "default_on_xml_error")]
    pub on_xml_error: OnError,

    #[serde(default = "default_on_scrap_error")]
    pub on_scrap_error: OnError,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            user_agent: String::from("SWSbot"),
            page_buffer: 10_000,
            throttle: None,
            num_workers: cmp::max(1, num_cpus::get().saturating_sub(2)),
            on_dl_error: OnError::SkipAndLog,
            on_xml_error: OnError::SkipAndLog,
            on_scrap_error: OnError::SkipAndLog,
        }
    }
}

fn default_user_agent() -> String {
    CrawlerConfig::default().user_agent
}

fn default_page_buffer() -> usize {
    CrawlerConfig::default().page_buffer
}

fn default_throttle() -> Option<Throttle> {
    CrawlerConfig::default().throttle
}

fn default_num_workers() -> usize {
    CrawlerConfig::default().num_workers
}

fn default_on_dl_error() -> OnError {
    CrawlerConfig::default().on_dl_error
}

fn default_on_xml_error() -> OnError {
    CrawlerConfig::default().on_xml_error
}

fn default_on_scrap_error() -> OnError {
    CrawlerConfig::default().on_scrap_error
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ArgEnum))]
pub enum OnError {
    Fail,
    SkipAndLog,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Throttle {
    /// The maximum number of concurrent requests
    Concurrent(NonZeroUsize),
    /// The number of requests per second
    PerSecond(NonZeroUsize),
    /// The delay in seconds between requests
    Delay(f32),
}

impl Default for Throttle {
    fn default() -> Self {
        Self::Concurrent(100.try_into().unwrap())
    }
}
