use clap::{Parser, Subcommand};

const LONG_ABOUT: &str = "\
Lycan turns websites into lightweight desktop apps on Linux using WebKit (wry + GTK).

Without a subcommand, the interactive TUI starts so you can add, edit, open, or remove PWAs.

Data lives under the XDG data directory (typically ~/.local/share/lycan/).";

const AFTER_HELP: &str = "\
Examples:
  lycan                      Start the TUI (manage PWAs)
  lycan open app-name       Open the PWA whose name is `app-name`
  lycan detect-system        Re-scan GPU / session and rewrite webkit-tuning.json for optimization";

#[derive(Parser)]
#[command(
    name = "lycan",
    version,
    about = "Lightweight PWA manager for Linux (WebKit / wry)",
    long_about = LONG_ABOUT,
    after_help = AFTER_HELP,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Re-detect GPU and session type, then rewrite WebKit tuning file
    ///
    /// Writes or updates ~/.local/share/lycan/webkit-tuning.json (DMA-BUF / compositing hints).
    /// Run this after switching GPU, drivers, or between X11 and Wayland if rendering feels off.
    DetectSystem,

    /// Open a saved PWA in its own window (session data under the app folder)
    ///
    /// The app id matches ~/.local/share/lycan/apps/<app-id>/ and the name used in generated .desktop files.
    Open {
        /// PWA application id (directory name under ~/.local/share/lycan/apps/)
        app_id: String,
    },
}
