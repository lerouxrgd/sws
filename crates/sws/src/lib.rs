use std::io;
use std::path::PathBuf;

use structopt::{clap::AppSettings, clap::Shell, StructOpt};
use tokio::runtime;

mod config;
mod crawler;
mod lua;

// TODO: switch to full clap

/// Scrap websites content
#[derive(Debug, StructOpt)]
pub struct ScrapOpts {
    /// Path where the scraping csv file will be written
    #[structopt(parse(from_os_str))]
    pub output_file: PathBuf,
    /// Path to the Lua script that defines scraping logic
    #[structopt(parse(from_os_str))]
    pub script_file: PathBuf,
}

#[derive(Debug, StructOpt)]
#[structopt()]
pub enum Command {
    #[structopt(name = "scrap")]
    Scrap(ScrapOpts),
    #[structopt(setting(AppSettings::Hidden))]
    Completion,
    #[structopt(setting(AppSettings::Hidden))]
    Help,
}

/// Sitemap Web Scraper
#[derive(Debug, StructOpt)]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: Command,
}

pub fn run_scrap(opts: ScrapOpts) -> anyhow::Result<()> {
    let rt = runtime::Builder::new_multi_thread().enable_all().build()?;

    let conf = lua::LuaScraperConfig {
        script: opts.script_file,
        csv_file: opts.output_file,
    };

    rt.block_on(crawler::crawl_site::<lua::LuaScraper>(&conf))?;

    Ok(())
}
