mod cli;
mod pwa;
mod runner;
mod tui;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        Some(cli::Commands::Open { app_id }) => runner::open(&app_id),
        None => tui::run(),
    }
}
