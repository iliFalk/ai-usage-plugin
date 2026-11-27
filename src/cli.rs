//! Command line — the whole surface the Omarchy panel drives.

use clap::{Parser, Subcommand, ValueEnum};

/// `ai-usagebar` — the data source for the Omarchy AI Usage panel.
#[derive(Debug, Parser)]
#[command(
    name = "ai-usagebar",
    version,
    about = "Quota and balance report for the Omarchy AI Usage panel",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Report every configured provider's usage, in one pass.
    Usage {
        /// Machine-readable document (the panel's contract).
        #[arg(long)]
        json: bool,
    },
    /// Read and write the configuration the panel edits.
    Settings {
        #[command(subcommand)]
        action: SettingsAction,
    },
}

#[derive(Debug, Clone, Subcommand, ValueEnum)]
pub enum SettingsAction {
    /// Print the non-secret settings snapshot as JSON.
    Show,
    /// Apply a JSON patch read from stdin.
    Apply,
}
