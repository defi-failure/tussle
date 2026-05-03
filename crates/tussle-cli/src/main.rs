use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tussle_core::sources::symbolichotkeys::SymbolicHotkeys;
use tussle_core::{Binding, BindingSource, Source};

#[derive(Parser)]
#[command(name = "tussle", version, about = "macOS hotkey conflict resolver")]
struct Cli {
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan { json } => scan(json),
    }
}

fn scan(as_json: bool) -> Result<()> {
    let path = system_symbolichotkeys_path()?;
    let bindings = SymbolicHotkeys::new(path.clone())
        .scan()
        .with_context(|| format!("scanning {}", path.display()))?;

    if as_json {
        let rows: Vec<BindingJson> = bindings.iter().map(BindingJson::from).collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if bindings.is_empty() {
        println!("(no customized system shortcuts found)");
        return Ok(());
    }

    let mut builder = Builder::default();
    builder.push_record(["Combo", "Owner", "Action"]);
    for b in &bindings {
        builder.push_record([&format!("{}", b.combo), b.source.owner(), &b.label]);
    }
    println!("{}", builder.build().with(Style::psql()));
    Ok(())
}

fn system_symbolichotkeys_path() -> Result<PathBuf> {
    Ok(dirs::preference_dir()
        .context("could not locate user preferences directory")?
        .join("com.apple.symbolichotkeys.plist"))
}

#[derive(Serialize)]
struct BindingJson<'a> {
    combo: String,
    owner: &'a str,
    action: &'a str,
    source: SourceJson,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SourceJson {
    SystemSymbolicHotkey {
        id: u32,
    },
    AppMenuOverride {
        bundle_id: String,
        menu_item: String,
    },
    AppMenuItem {
        bundle_id: String,
        app_name: Option<String>,
        menu_path: Vec<String>,
    },
}

impl<'a> From<&'a Binding> for BindingJson<'a> {
    fn from(b: &'a Binding) -> Self {
        Self {
            combo: format!("{}", b.combo),
            owner: b.source.owner(),
            action: &b.label,
            source: match &b.source {
                BindingSource::SystemSymbolicHotkey { id } => {
                    SourceJson::SystemSymbolicHotkey { id: *id }
                }
                BindingSource::AppMenuOverride {
                    bundle_id,
                    menu_item,
                } => SourceJson::AppMenuOverride {
                    bundle_id: bundle_id.clone(),
                    menu_item: menu_item.clone(),
                },
                BindingSource::AppMenuItem {
                    bundle_id,
                    app_name,
                    menu_path,
                } => SourceJson::AppMenuItem {
                    bundle_id: bundle_id.clone(),
                    app_name: app_name.clone(),
                    menu_path: menu_path.clone(),
                },
            },
        }
    }
}
