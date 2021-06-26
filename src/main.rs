use std::io;
use std::path::PathBuf;

use structopt::{clap::AppSettings, clap::Shell, StructOpt};
use tokio::runtime;

mod scraper;
mod urbandict;

/// Scrap websites content
#[derive(Debug, StructOpt)]
pub struct ScrapOpts {
    /// Path where the urbandict scraping tsv file will be written
    #[structopt(parse(from_os_str))]
    pub urbandict_tsv: PathBuf,
}

#[derive(Debug, StructOpt)]
#[structopt()]
pub enum Command {
    #[structopt(name = "scrap")]
    Scrap(ScrapOpts),
    #[structopt(name = "clean")]
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
    use scraper::scrap_site;
    use urbandict::Urbandict;

    let rt = runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(scrap_site(Urbandict::new(opts.urbandict_tsv)?))
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
