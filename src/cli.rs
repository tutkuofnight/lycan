use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lycan", about = "Lightweight PWA manager for i3/Hyprland")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Open a PWA by its app ID
    Open {
        /// The app ID to open
        app_id: String,
    },
}
