use std::io;
use std::path::PathBuf;

use structopt::{clap::AppSettings, clap::Shell, StructOpt};
use sws_crawler::crawl_site;
use sws_lua::{LuaScraper, LuaScraperConfig};
use tokio::runtime;

// TODO: switch to full clap

/// Scrap website content
#[derive(Debug, StructOpt)]
pub struct ScrapOpts {
    /// Path to the Lua script that defines scraping logic
    #[structopt(parse(from_os_str))]
    pub script_file: PathBuf,
    /// Path where the scraping csv file will be written
    #[structopt(parse(from_os_str))]
    pub output_file: PathBuf,
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

    let conf = LuaScraperConfig {
        script: opts.script_file,
        csv_file: opts.output_file,
    };

    rt.block_on(crawl_site::<LuaScraper>(&conf))?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    match Opts::from_args().command {
        Command::Scrap(opts) => run_scrap(opts),
        Command::Completion => {
            Opts::clap().gen_completions_to("sws", Shell::Bash, &mut io::stdout());
            Ok(())
        }
        Command::Help => Ok(()),
    }
}
