use std::io;

use structopt::{clap::AppSettings, clap::Shell, StructOpt};
use sws::{run_scrap, Command, Opts, ScrapOpts};

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
