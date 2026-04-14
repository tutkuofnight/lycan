mod cli;
mod fs_private;
mod pwa;
mod runner;
mod tui;
mod webkit_tuning;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        Some(cli::Commands::DetectSystem) => webkit_tuning::regenerate_profile(),
        Some(cli::Commands::Open { app_id }) => {
            webkit_tuning::init();
            runner::open(&app_id)
        }
        None => {
            webkit_tuning::init();
            tui::run()
        }
    }
}
