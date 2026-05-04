//! Command-line interface: argument parsing and command dispatch.

mod commands;
mod output;
mod sources;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tussle", version, about = "macOS hotkey conflict resolver")]
struct Cli {
    /// Per-app Accessibility messaging timeout, in seconds. Caps how long
    /// a single non-responsive app can stall the scan. Set to `0` to use
    /// the macOS default (~6s).
    #[arg(long, global = true, default_value_t = 1.0, value_name = "SECS")]
    ax_timeout: f32,

    /// Defensive cap on the number of apps walked in parallel. `0` means
    /// no cap (one OS thread per app, all at once). Default 128 — at the
    /// typical 50–100 running apps this is effectively unbounded; set
    /// lower only if a session has hundreds of processes and you'd rather
    /// pay extra wallclock than hold them all open at once.
    #[arg(long, global = true, default_value_t = 128, value_name = "N")]
    ax_concurrency: usize,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan all hotkey sources and print discovered bindings.
    Scan {
        /// Emit JSON instead of a human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Look up which sources own a key combination.
    Who {
        /// Combo to look up, e.g. `cmd+opt+b`. Omit to enter interactive
        /// capture mode.
        combo: Option<String>,

        /// Emit JSON instead of a human-readable table.
        #[arg(long)]
        json: bool,
    },
}

/// Parse argv and dispatch to the chosen subcommand.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan { json } => commands::scan::scan(json, cli.ax_timeout, cli.ax_concurrency),
        Command::Who { combo, json } => {
            commands::who::who(combo, json, cli.ax_timeout, cli.ax_concurrency)
        }
    }
}
