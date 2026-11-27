//! ai-usagebar library — the data source behind the Omarchy AI Usage panel.
//!
//! The crate is organized by concern, not by command:
//! - low-level primitives (`cache`, `countdown`, `pacing`, `format`, `display`)
//! - the provider abstraction (`vendor`, `opencode_go`, `openrouter`,
//!   `deepseek`, `usage`)
//! - the provider client (`provider`) that turns config into entries and
//!   fetches them, and the projection (`sections`) that flattens a snapshot
//!   into the rows a frontend draws
//! - the two commands: `report` (`usage --json`) and `settings`

pub mod balance;
pub mod cache;
pub mod cli;
pub mod config;
pub mod countdown;
pub mod deepseek;
pub mod display;
pub mod error;
pub mod format;
/// Source-scanning helpers for structural guard tests. Test-only.
#[cfg(test)]
pub(crate) mod guard;
pub mod notify;
pub mod opencode_go;
pub mod openrouter;
pub mod outcome;
pub mod pacing;
pub mod provider;
pub mod report;
pub mod sections;
pub mod serde_helpers;
pub mod settings;
pub mod usage;
pub mod vendor;

pub use error::{AppError, Result};
