use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tussle_core::capture::{self, Captured};
use tussle_core::sources::accessibility::{self, Accessibility};
use tussle_core::sources::nsuserkeyequivalents::AppMenuOverrides;
use tussle_core::sources::symbolichotkeys::SymbolicHotkeys;
use tussle_core::{Binding, BindingSource, KeyCombo, Source};

#[derive(Parser)]
#[command(name = "tussle", version, about = "macOS hotkey conflict resolver")]
struct Cli {
    /// Per-app Accessibility messaging timeout, in seconds. Caps how long
    /// a single non-responsive app can stall the scan. Set to `0` to use
    /// the macOS default (~6s).
    #[arg(long, global = true, default_value_t = 1.0, value_name = "SECS")]
    ax_timeout: f32,

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
        /// capture mode (not yet implemented).
        combo: Option<String>,

        /// Emit JSON instead of a human-readable table.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan { json } => scan(json, cli.ax_timeout),
        Command::Who { combo, json } => who(combo, json, cli.ax_timeout),
    }
}

fn scan(as_json: bool, ax_timeout: f32) -> Result<()> {
    let sources = default_sources(ax_timeout)?;

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

fn who(combo_arg: Option<String>, as_json: bool, ax_timeout: f32) -> Result<()> {
    let combo = match combo_arg {
        Some(text) => KeyCombo::parse(&text).with_context(|| format!("parsing combo {text:?}"))?,
        None => {
            eprintln!("Press the hotkey to look up (Ctrl+C to abort)...");
            let captured = capture::capture_one(|mods| {
                use std::io::Write;
                let mut stderr = std::io::stderr().lock();
                // \x1B[2K clears the entire line; \r returns the cursor.
                if mods.is_empty() {
                    let _ = write!(stderr, "\r\x1B[2K");
                } else {
                    let _ = write!(stderr, "\r\x1B[2KHolding: {mods}+");
                }
                let _ = stderr.flush();
            })
            .context("capturing keystroke")?;
            match captured {
                Captured::Combo(c) => {
                    eprintln!("\r\x1B[2KCaptured: {c} — looking up...");
                    c
                }
                Captured::SystemAction(action) => {
                    eprintln!(
                        "\r\x1B[2KCaptured: vk 0x{:02x} — '{}'.",
                        action.vk,
                        action.kind.name(),
                    );
                    eprintln!(
                        "This is a macOS system action: dispatched by macOS itself, \
                         not an app-bindable hotkey. Apple does not document the \
                         0x80+ virtual-keycode range (kVK_* tops out at 0x7E)."
                    );
                    if let Some(hint) = action.kind.source_hint() {
                        eprintln!("To change it: {hint}.");
                    }
                    return Ok(());
                }
            }
        }
    };

    let sources = default_sources(ax_timeout)?;

    if !accessibility::is_trusted() {
        eprintln!(
            "note: tussle does not currently have Accessibility permission, \
             so app menu shortcuts will be missing. Grant access in \
             System Settings → Privacy & Security → Accessibility, then re-run."
        );
    }

    let mut matches: Vec<Binding> = Vec::new();
    for src in &sources {
        match src.scan() {
            Ok(found) => matches.extend(found.into_iter().filter(|b| b.combo == combo)),
            Err(e) => eprintln!("{}: {:#}", src.name(), e),
        }
    }

    if as_json {
        let rows: Vec<BindingJson> = matches.iter().map(BindingJson::from).collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if matches.is_empty() {
        println!("nothing bound to {combo}");
        return Ok(());
    }

    let mut builder = Builder::default();
    builder.push_record(["Owner", "Action"]);
    for b in &matches {
        builder.push_record([b.source.owner(), b.label.as_str()]);
    }
    println!();
    println!("{}", builder.build().with(Style::psql()));
    Ok(())
}

/// Build the default macOS source set.
///
/// Each source is constructed with paths/configuration the CLI looks up via
/// `dirs`; `tussle-core` itself stays filesystem-agnostic.
fn default_sources(ax_timeout: f32) -> Result<Vec<Box<dyn Source>>> {
    let prefs = dirs::preference_dir().context("could not locate user preferences directory")?;

    Ok(vec![
        Box::new(SymbolicHotkeys::new(
            prefs.join("com.apple.symbolichotkeys.plist"),
        )),
        Box::new(AppMenuOverrides::new(prefs.clone())),
        Box::new(Accessibility::new(ax_timeout)),
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
