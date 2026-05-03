use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tussle_core::sources::symbolichotkeys;

#[derive(Parser)]
#[command(name = "tussle", version, about = "macOS hotkey conflict resolver")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan all hotkey sources and print discovered bindings.
    Scan,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan => scan(),
    }
}

fn scan() -> Result<()> {
    let path = system_symbolichotkeys_path()?;
    let bindings = symbolichotkeys::scan(&path)
        .with_context(|| format!("scanning {}", path.display()))?;

    if bindings.is_empty() {
        println!("(no customized system shortcuts found)");
        return Ok(());
    }

    for b in &bindings {
        println!("{}\t{}\t{}", b.combo, b.source.owner(), b.label);
    }
    Ok(())
}

fn system_symbolichotkeys_path() -> Result<PathBuf> {
    Ok(dirs::preference_dir()
        .context("could not locate user preferences directory")?
        .join("com.apple.symbolichotkeys.plist"))
}
