use std::io;
use std::path::PathBuf;

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use sws_crawler::crawl_site;
use sws_lua::{LuaScraper, LuaScraperConfig};
use tokio::runtime;

/// Sitemap Web Scraper
#[derive(Debug, Parser)]
#[clap(version)]
pub struct Args {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum SubCommand {
    #[clap(name = "scrap")]
    Scrap(ScrapArgs),
    #[clap(hide = true)]
    Completion,
}

/// Scrap website content
#[derive(Debug, clap::Args)]
pub struct ScrapArgs {
    /// Path to the Lua script that defines scraping logic
    #[clap(parse(from_os_str), long, short)]
    pub script: PathBuf,
    /// Path to the output file that will contain scrapped data
    #[clap(parse(from_os_str), long, short)]
    pub output_file: PathBuf,
}

pub fn scrap(args: ScrapArgs) -> anyhow::Result<()> {
    let scraper_conf = LuaScraperConfig {
        script: args.script,
        csv_file: args.output_file,
    };
    let rt = runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(crawl_site::<LuaScraper>(&scraper_conf))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.cmd {
        SubCommand::Scrap(args) => scrap(args),
        SubCommand::Completion => {
            generate(Shell::Bash, &mut Args::command(), "sws", &mut io::stdout());
            Ok(())
        }
    }
}
