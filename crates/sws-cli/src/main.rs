use std::fs::File;
use std::path::PathBuf;
use std::{cmp, env, io};

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use sws_crawler::{crawl_site, CrawlerConfig, OnError, PageLocation, Scrapable};
use sws_lua::{scrap_glob, scrap_page, LuaScraper, LuaScraperConfig};
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
    #[clap(name = "crawl")]
    Crawl(CrawlArgs),
    #[clap(name = "scrap")]
    Scrap(ScrapArgs),
    #[clap(hide = true)]
    Completion,
}

/// Crawl sitemap and scrap pages content
#[derive(Debug, clap::Args)]
pub struct CrawlArgs {
    /// Path to the Lua script that defines scraping logic
    #[clap(display_order(1), parse(from_os_str), long, short)]
    pub script: PathBuf,

    /// Output file that will contain scrapped data
    #[clap(display_order(2), parse(from_os_str), long, short)]
    pub output_file: PathBuf,

    /// Override crawler's user agent
    #[clap(display_order(4), long)]
    pub user_agent: Option<String>,

    /// Override crawler's page buffer size
    #[clap(display_order(5), long)]
    pub page_buffer: Option<usize>,

    /// Override crawler's maximum concurrent downloads for pages
    #[clap(display_order(6), long = "conc-dl")]
    pub concurrent_downloads: Option<usize>,

    /// Override crawler's number of CPU workers used to scrap pages
    #[clap(display_order(7), long)]
    pub num_workers: Option<usize>,

    /// Override crawler's download error handling strategy
    #[clap(display_order(9), arg_enum, long)]
    pub on_dl_error: Option<OnError>,

    /// Override crawler's xml error handling strategy
    #[clap(display_order(10), arg_enum, long)]
    pub on_xml_error: Option<OnError>,

    /// Override crawler's scrap error handling strategy
    #[clap(display_order(11), arg_enum, long)]
    pub on_scrap_error: Option<OnError>,

    /// Don't output logs
    #[clap(display_order(12), long, short)]
    pub quiet: bool,
}

pub fn crawl(args: CrawlArgs) -> anyhow::Result<()> {
    let scraper_conf = LuaScraperConfig {
        script: args.script,
        csv_file: args.output_file,
    };

    let mut crawler_conf = CrawlerConfig::try_from(&scraper_conf)?;
    if let Some(user_agent) = &args.user_agent {
        crawler_conf.user_agent = user_agent.to_string();
    }
    if let Some(page_buffer) = args.page_buffer {
        crawler_conf.page_buffer = page_buffer;
    }
    if let Some(concurrent_downloads) = args.concurrent_downloads {
        crawler_conf.concurrent_downloads = concurrent_downloads;
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

    let rt = runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(crawl_site::<LuaScraper>(&crawler_conf, &scraper_conf))
}

/// Scrap a single page and print the result to stdout
#[derive(Debug, clap::Args)]
#[clap(group = clap::ArgGroup::new("pages").required(true))]
pub struct ScrapArgs {
    /// Path to the Lua script that defines scraping logic
    #[clap(display_order(1), parse(from_os_str), long, short)]
    pub script: PathBuf,

    /// A distant html page to scrap
    #[clap(display_order(2), group = "pages", long)]
    pub url: Option<String>,

    /// A glob pattern to select local files to scrap
    #[clap(display_order(3), group = "pages", long)]
    pub glob: Option<String>,

    /// Scrap error handling strategy when scraping glob files
    #[clap(display_order(4), arg_enum, long)]
    #[clap(conflicts_with = "url")]
    pub on_error: Option<OnError>,

    /// Set the number of CPU workers when scraping glob files
    #[clap(display_order(5), long)]
    #[clap(conflicts_with = "url")]
    pub num_workers: Option<usize>,

    /// Optional file that will contain scrapped data, stdout otherwise
    #[clap(display_order(6), parse(from_os_str), long, short)]
    pub output_file: Option<PathBuf>,

    /// Don't output logs
    #[clap(display_order(7), long, short)]
    pub quiet: bool,
}

pub fn scrap(args: ScrapArgs) -> anyhow::Result<()> {
    let (output_file, temp_file) = if let Some(output_file) = args.output_file {
        (output_file, None)
    } else {
        let temp_file = tempfile::NamedTempFile::new()?;
        let output_file = temp_file.path().into();
        (output_file, Some(temp_file))
    };

    let config = LuaScraperConfig {
        script: args.script,
        csv_file: output_file.clone(),
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
            let num_workers = args.num_workers.unwrap_or(cmp::max(1, num_cpus::get()));
            let on_error = args.on_error.unwrap_or(OnError::Fail);
            let mut scraper = LuaScraper::new(&config)?;
            let res = scrap_glob(&config, &pattern, on_error, num_workers);
            scraper.finalizer();
            res?;
        }
        _ => anyhow::bail!("Invalid arguments"),
    }

    if temp_file.is_some() {
        let mut reader = io::BufReader::new(File::open(&output_file)?);
        io::copy(&mut reader, &mut io::stdout())?;
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
