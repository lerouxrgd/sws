use std::path::PathBuf;
use std::{cmp, env, io};

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use sws_crawler::{crawl_site, CrawlerConfig, OnError, PageLocation, Scrapable, Throttle};
use sws_lua::{scrap_glob, scrap_page, writer::FileMode, LuaScraper, LuaScraperConfig};
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
    #[clap(display_order(1), name = "crawl")]
    Crawl(CrawlArgs),
    #[clap(display_order(2), name = "scrap")]
    Scrap(ScrapArgs),
    #[clap(hide = true)]
    Completion,
}

/// Crawl sitemaps and scrap pages content
#[derive(Debug, clap::Args)]
#[clap(group = clap::ArgGroup::new("mode").requires_all(&["output-file"]))]
#[clap(group = clap::ArgGroup::new("throttle"))]
pub struct CrawlArgs {
    /// Path to the Lua script that defines scraping logic
    #[clap(display_order(1), long, short)]
    pub script: PathBuf,

    /// Optional file that will contain scraped data, stdout otherwise
    #[clap(display_order(2), long, short)]
    pub output_file: Option<PathBuf>,

    /// Append to output file
    #[clap(display_order(3), group = "mode", long)]
    pub append: bool,

    /// Truncate output file
    #[clap(display_order(4), group = "mode", long)]
    pub truncate: bool,

    /// Override crawler's user agent
    #[clap(display_order(5), long)]
    pub user_agent: Option<String>,

    /// Override crawler's page buffer size
    #[clap(display_order(6), long)]
    pub page_buffer: Option<usize>,

    /// Override crawler's maximum concurrent downloads for pages
    #[clap(display_order(7), group = "throttle", long = "conc-dl")]
    pub concurrent_downloads: Option<usize>,

    /// Override crawler's number of requests per second
    #[clap(display_order(8), group = "throttle", long = "rps")]
    pub requests_per_second: Option<usize>,

    /// Override crawler's delay between requests
    #[clap(display_order(9), group = "throttle", long = "delay", value_parser = delay_positive)]
    pub requests_delay: Option<f32>,

    /// Override crawler's number of CPU workers used to scrap pages
    #[clap(display_order(10), long)]
    pub num_workers: Option<usize>,

    /// Override crawler's download error handling strategy
    #[clap(display_order(11), value_enum, long)]
    pub on_dl_error: Option<OnError>,

    /// Override crawler's xml error handling strategy
    #[clap(display_order(12), value_enum, long)]
    pub on_xml_error: Option<OnError>,

    /// Override crawler's scrap error handling strategy
    #[clap(display_order(13), value_enum, long)]
    pub on_scrap_error: Option<OnError>,

    /// Override crawler's robots.txt URL
    #[clap(display_order(14), long)]
    pub robot: Option<String>,

    /// Don't output logs
    #[clap(display_order(15), long, short)]
    pub quiet: bool,
}

fn delay_positive(s: &str) -> Result<f32, String> {
    let delay: f32 = s
        .parse()
        .map_err(|_| format!("`{}` isn't a f32 value", s))?;
    if delay > 0. {
        Ok(delay)
    } else {
        Err("delay must be strictly positive".into())
    }
}

pub fn crawl(args: CrawlArgs) -> anyhow::Result<()> {
    let file_mode = if args.append {
        Some(FileMode::Append)
    } else if args.truncate {
        Some(FileMode::Truncate)
    } else {
        None
    };

    let scraper_conf = LuaScraperConfig {
        script: args.script,
        csv_file: args.output_file,
        file_mode,
    };

    let mut crawler_conf = CrawlerConfig::try_from(&scraper_conf)?;
    if let Some(user_agent) = &args.user_agent {
        crawler_conf.user_agent = user_agent.to_string();
    }
    if let Some(page_buffer) = args.page_buffer {
        crawler_conf.page_buffer = page_buffer;
    }
    if let Some(conc_dl) = args.concurrent_downloads {
        crawler_conf.throttle = Some(Throttle::Concurrent(conc_dl.try_into()?));
    }
    if let Some(rps) = args.requests_per_second {
        crawler_conf.throttle = Some(Throttle::PerSecond(rps.try_into()?));
    }
    if let Some(delay) = args.requests_delay {
        crawler_conf.throttle = Some(Throttle::Delay(delay));
    }
    if let Some(num_workers) = args.num_workers {
        crawler_conf.num_workers = num_workers;
    }
    if let Some(on_dl_error) = args.on_dl_error {
        crawler_conf.on_dl_error = on_dl_error;
    }
    if let Some(on_xml_error) = args.on_xml_error {
        crawler_conf.on_xml_error = on_xml_error;
    }
    if let Some(on_scrap_error) = args.on_scrap_error {
        crawler_conf.on_scrap_error = on_scrap_error;
    }
    if let Some(robot) = args.robot {
        crawler_conf.robot = Some(robot);
    }

    let rt = runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(crawl_site::<LuaScraper>(&crawler_conf, &scraper_conf))
}

/// Scrap a single remote page or multiple local pages
#[derive(Debug, clap::Args)]
#[clap(group = clap::ArgGroup::new("pages").required(true))]
#[clap(group = clap::ArgGroup::new("mode").requires_all(&["output-file"]))]
pub struct ScrapArgs {
    /// Path to the Lua script that defines scraping logic
    #[clap(display_order(1), long, short)]
    pub script: PathBuf,

    /// A distant html page to scrap
    #[clap(display_order(2), group = "pages", long)]
    pub url: Option<String>,

    /// A glob pattern to select local files to scrap
    #[clap(display_order(3), group = "pages", long = "files")]
    pub glob: Option<String>,

    /// Optional file that will contain scraped data, stdout otherwise
    #[clap(display_order(4), long, short)]
    pub output_file: Option<PathBuf>,

    /// Append to output file
    #[clap(display_order(5), group = "mode", long)]
    pub append: bool,

    /// Truncate output file
    #[clap(display_order(6), group = "mode", long)]
    pub truncate: bool,

    /// Set the number of CPU workers when scraping local files
    #[clap(display_order(7), long)]
    #[clap(conflicts_with = "url")]
    pub num_workers: Option<usize>,

    /// Scrap error handling strategy when scraping local files
    #[clap(display_order(8), value_enum, long)]
    #[clap(conflicts_with = "url")]
    pub on_error: Option<OnError>,

    /// Don't output logs
    #[clap(display_order(9), long, short)]
    pub quiet: bool,
}

pub fn scrap(args: ScrapArgs) -> anyhow::Result<()> {
    let file_mode = if args.append {
        Some(FileMode::Append)
    } else if args.truncate {
        Some(FileMode::Truncate)
    } else {
        None
    };

    let config = LuaScraperConfig {
        script: args.script,
        csv_file: args.output_file,
        file_mode,
    };

    match (args.url, args.glob) {
        (Some(url), None) => {
            let ua = CrawlerConfig::try_from(&config)?.user_agent;
            let client = reqwest::blocking::ClientBuilder::new()
                .user_agent(ua)
                .build()?;
            let page = client.get(&url).send()?.text()?;
            scrap_page(&config, page, PageLocation::Url(url))?
        }
        (None, Some(pattern)) => {
            let num_workers = args
                .num_workers
                .unwrap_or_else(|| cmp::max(1, num_cpus::get()));
            let on_error = args.on_error.unwrap_or(OnError::Fail);
            let mut scraper = LuaScraper::new(&config)?;
            let res = scrap_glob(&config, &pattern, on_error, num_workers);
            scraper.finalizer();
            res?;
        }
        _ => anyhow::bail!("Invalid arguments"),
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.cmd {
        SubCommand::Crawl(args) => {
            if !args.quiet {
                env::set_var("RUST_LOG", "sws_lua=warn,sws_crawler=warn");
                env_logger::init();
            }
            crawl(args)
        }
        SubCommand::Scrap(args) => {
            if !args.quiet {
                env::set_var("RUST_LOG", "sws_lua=warn");
                env_logger::init();
            }
            scrap(args)
        }
        SubCommand::Completion => {
            generate(Shell::Bash, &mut Args::command(), "sws", &mut io::stdout());
            Ok(())
        }
    }
}
