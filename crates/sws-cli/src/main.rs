use std::fs::File;
use std::io;
use std::path::PathBuf;

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use sws_crawler::{crawl_site, CrawlerConfig};
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
    /// Optional default carwler yaml configuration file
    #[clap(env = "SWS_CRAWLER_CONFIG", parse(from_os_str), long)]
    pub crawler_config: Option<PathBuf>,
    /// Override crawler's user agent
    #[clap(long)]
    pub user_agent: Option<String>,
    /// Override crawler's page buffer size
    #[clap(long)]
    pub page_buffer: Option<usize>,
    /// Override crawler's maximum concurrent page downloads
    #[clap(long)]
    pub concurrent_downloads: Option<usize>,
    /// Override crawler's number of CPU workers used to parse pages
    #[clap(long)]
    pub num_workers: Option<usize>,
}

impl TryFrom<&ScrapArgs> for CrawlerConfig {
    type Error = anyhow::Error;

    fn try_from(args: &ScrapArgs) -> Result<Self, Self::Error> {
        let mut conf = if let Some(file) = args.crawler_config.as_ref().map(|p| File::open(p)) {
            serde_yaml::from_reader(file?)?
        } else {
            CrawlerConfig::default()
        };
        if let Some(user_agent) = &args.user_agent {
            conf.user_agent = user_agent.to_string();
        }
        if let Some(page_buffer) = args.page_buffer {
            conf.page_buffer = page_buffer;
        }
        if let Some(concurrent_downloads) = args.concurrent_downloads {
            conf.concurrent_downloads = concurrent_downloads;
        }
        if let Some(num_workers) = args.num_workers {
            conf.num_workers = num_workers;
        }
        Ok(conf)
    }
}

pub fn scrap(args: ScrapArgs) -> anyhow::Result<()> {
    let crawler_conf = (&args).try_into()?;
    let scraper_conf = LuaScraperConfig {
        script: args.script,
        csv_file: args.output_file,
    };
    let rt = runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(crawl_site::<LuaScraper>(&crawler_conf, &scraper_conf))
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
