use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tussle_core::sources::accessibility::{self, Accessibility};
use tussle_core::sources::nsuserkeyequivalents::AppMenuOverrides;
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
    let sources = default_sources()?;

    if !accessibility::is_trusted() {
        eprintln!(
            "note: tussle does not currently have Accessibility permission, \
             so app menu shortcuts will be missing. Grant access in \
             System Settings → Privacy & Security → Accessibility, then re-run."
        );
    }

    let mut bindings: Vec<Binding> = Vec::new();
    for src in &sources {
        match src.scan() {
            Ok(found) => bindings.extend(found),
            Err(e) => eprintln!("{}: {:#}", src.name(), e),
        }
    }

    if as_json {
        let rows: Vec<BindingJson> = bindings.iter().map(BindingJson::from).collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if bindings.is_empty() {
        println!("(no bindings found)");
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

/// Build the default macOS source set.
///
/// Each source is constructed with paths/configuration the CLI looks up via
/// `dirs`; `tussle-core` itself stays filesystem-agnostic.
fn default_sources() -> Result<Vec<Box<dyn Source>>> {
    let prefs = dirs::preference_dir()
        .context("could not locate user preferences directory")?;

    Ok(vec![
        Box::new(SymbolicHotkeys::new(
            prefs.join("com.apple.symbolichotkeys.plist"),
        )),
        Box::new(AppMenuOverrides::new(prefs.clone())),
        Box::new(Accessibility),
    ])
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
