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
            user_agent: default_user_agent(),
            page_buffer: default_page_buffer(),
            throttle: default_throttle(),
            num_workers: default_num_workers(),
            on_dl_error: default_on_dl_error(),
            on_xml_error: default_on_xml_error(),
            on_scrap_error: default_on_scrap_error(),
        }
    }
}

fn default_user_agent() -> String {
    String::from("SWSbot")
}

fn default_page_buffer() -> usize {
    10_000
}

fn default_throttle() -> Option<Throttle> {
    None
}

fn default_num_workers() -> usize {
    cmp::max(1, num_cpus::get().saturating_sub(2))
}

fn default_on_dl_error() -> OnError {
    OnError::SkipAndLog
}

fn default_on_xml_error() -> OnError {
    OnError::SkipAndLog
}

fn default_on_scrap_error() -> OnError {
    OnError::SkipAndLog
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
