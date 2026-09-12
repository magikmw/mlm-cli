//! Argument parsing (clap).

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mlm", version, about = "Quick time tracking from the CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Log a start point for today.
    Start {
        /// Optional note about what you're starting.
        note: Option<String>,
    },
    /// Log a stop point for today.
    Stop {
        /// Optional note about what you finished.
        note: Option<String>,
    },
    /// Show today's (or a given date's) log.
    Log {
        /// Date to show, defaults to today. Format: YYYY-MM-DD.
        date: Option<String>,
    },
}
