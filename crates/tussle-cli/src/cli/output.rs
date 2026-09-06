//! Serializable shapes for the `--json` output mode and the helper that
//! prints them. Table rendering stays inside each command so column choices
//! live next to the command that uses them.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use serde::Serialize;
use tabled::builder::Builder;
use tabled::settings::peaker::Priority;
use tabled::settings::{Style, Width};
use tussle_core::{Binding, BindingSource, HotkeyIndex, Layer, SystemDispatch, Winner};

#[derive(Serialize)]
pub(super) struct BindingJson<'a> {
    combo: String,
    owner: &'a str,
    action: &'a str,
    layer: &'static str,
    enabled: bool,
    source: SourceJson,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SourceJson {
    SystemSymbolicHotkey {
        id: Option<u32>,
        dispatch: &'static str,
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
    StatusMenuItem {
        bundle_id: String,
        app_name: Option<String>,
        menu_path: Vec<String>,
    },
    AppleMenuItem {
        menu_path: Vec<String>,
    },
}

impl<'a> From<&'a Binding> for BindingJson<'a> {
    fn from(b: &'a Binding) -> Self {
        Self {
            combo: format!("{}", b.combo),
            owner: b.source.owner(),
            action: &b.label,
            layer: b.source.layer().name(),
            enabled: b.enabled,
            source: match &b.source {
                BindingSource::SystemSymbolicHotkey { id, dispatch } => {
                    SourceJson::SystemSymbolicHotkey {
                        id: *id,
                        dispatch: match dispatch {
                            SystemDispatch::BeforeApps => "before_apps",
                            SystemDispatch::StandardMenuItem => "standard_menu_item",
                        },
                    }
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
                BindingSource::StatusMenuItem {
                    bundle_id,
                    app_name,
                    menu_path,
                } => SourceJson::StatusMenuItem {
                    bundle_id: bundle_id.clone(),
                    app_name: app_name.clone(),
                    menu_path: menu_path.clone(),
                },
                BindingSource::AppleMenuItem { menu_path } => SourceJson::AppleMenuItem {
                    menu_path: menu_path.clone(),
                },
            },
        }
    }
}

/// Who gets a combo, as reported by `tussle who --json`.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum VerdictJson<'a> {
    Nobody,
    Global { binding: BindingJson<'a> },
    Contested { layer: &'static str },
    FrontmostApp,
}

impl<'a> From<Winner<'a>> for VerdictJson<'a> {
    fn from(w: Winner<'a>) -> Self {
        match w {
            Winner::Nobody => VerdictJson::Nobody,
            Winner::Global(b) => VerdictJson::Global {
                binding: BindingJson::from(b),
            },
            Winner::Contested(layer) => VerdictJson::Contested {
                layer: layer.name(),
            },
            Winner::FrontmostApp => VerdictJson::FrontmostApp,
        }
    }
}

/// Force machine-readable table output even on a terminal (`--plain`).
static PLAIN: AtomicBool = AtomicBool::new(false);

pub(super) fn set_plain(plain: bool) {
    PLAIN.store(plain, Ordering::Relaxed);
}

/// Whether table output goes to a person (aligned, headed, fitted to the
/// terminal) or to a program (tab-separated, no header, nothing cut).
/// The heuristic is the one in clig.dev and GitHub's CLI: stdout being a
/// terminal, unless `--plain` was given.
pub(super) fn human_output() -> bool {
    !PLAIN.load(Ordering::Relaxed) && std::io::stdout().is_terminal()
}

/// Print a table the way GitHub's CLI does. On a terminal: a header,
/// aligned columns, and cells truncated with "…" so every row fits the
/// terminal width, widest column first. Piped, or with `--plain`: one
/// row per line, tab-separated, no header, nothing truncated, so `cut`,
/// `awk` and `grep` see the complete data.
pub(super) fn print_table(header: &[&str], rows: &[Vec<String>]) {
    if !human_output() {
        print!("{}", tsv(rows));
        return;
    }
    let mut builder = Builder::default();
    builder.push_record(header.iter().copied());
    for row in rows {
        builder.push_record(row);
    }
    let mut table = builder.build();
    table.with(Style::psql());
    if let Some((terminal_size::Width(cols), _)) = terminal_size::terminal_size() {
        table.with(
            Width::truncate(usize::from(cols))
                .suffix("…")
                .priority(Priority::max(true)),
        );
    }
    println!("{table}");
}

/// Tab-separated rows, one per line. Tabs and newlines inside a cell
/// would break the format, so they become spaces.
pub(super) fn tsv(rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    for row in rows {
        let cells: Vec<String> = row
            .iter()
            .map(|c| c.replace(['\t', '\n', '\r'], " "))
            .collect();
        out.push_str(&cells.join("\t"));
        out.push('\n');
    }
    out
}

/// Tell a person that a command found nothing. Goes to stderr, and only
/// when a person is reading: a script gets an empty stdout and exit 0,
/// because no results is not a failure.
pub(super) fn no_results(message: &str) {
    if human_output() {
        eprintln!("{message}");
    }
}

/// Print any serializable value as pretty-printed JSON to stdout.
pub(super) fn emit_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Print the scan's partial-result warnings to stderr, one per line, so a
/// user can tell an incomplete answer from a complete one.
pub(super) fn report_warnings(index: &HotkeyIndex) {
    for w in index.warnings() {
        eprintln!("note: {w}");
    }
}

/// Human-readable name of a layer for tables.
pub(super) fn layer_label(layer: Layer) -> &'static str {
    layer.name()
}

/// Where a person would go to change or remove `b`.
pub(super) fn how_to_change(b: &Binding) -> String {
    const PANE: &str = "System Settings → Keyboard → Keyboard Shortcuts";
    match &b.source {
        BindingSource::SystemSymbolicHotkey {
            id: Some(id),
            dispatch: SystemDispatch::BeforeApps,
        } => format!("{PANE} → {}", settings_section(*id)),
        BindingSource::SystemSymbolicHotkey {
            id: None,
            dispatch: SystemDispatch::BeforeApps,
        } => "built into macOS; it cannot be changed".to_string(),
        BindingSource::SystemSymbolicHotkey {
            dispatch: SystemDispatch::StandardMenuItem,
            ..
        } => format!("a standard menu item; {PANE} → App Shortcuts can override it for one app"),
        BindingSource::AppleMenuItem { .. } => "built into macOS; it cannot be changed".to_string(),
        BindingSource::StatusMenuItem { .. } => {
            format!("in {}'s own settings", b.source.owner())
        }
        BindingSource::AppMenuItem { .. } => format!(
            "in {} itself if it lets you, otherwise {PANE} → App Shortcuts: add {} with the menu title \"{}\"",
            b.source.owner(),
            b.source.owner(),
            b.label
        ),
        BindingSource::AppMenuOverride { bundle_id, .. } => {
            format!("{PANE} → App Shortcuts: this is your own override for {bundle_id}")
        }
    }
}

/// Section of the Keyboard Shortcuts pane that lists symbolic hotkey `id`.
fn settings_section(id: u32) -> &'static str {
    match id {
        7..=13 | 27 | 51 | 57 | 98 => "Keyboard",
        15..=26 | 59 | 175 => "Accessibility",
        28..=31 | 181 | 182 | 184 => "Screenshots",
        32 | 33 | 36 | 79..=82 | 118..=121 => "Mission Control",
        52 | 160 => "Launchpad & Dock",
        60 | 61 => "Input Sources",
        64 | 65 => "Spotlight",
        _ => "the section listing it",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsv_is_one_row_per_line_with_cells_sanitised() {
        let rows = vec![
            vec![
                "cmd+w".to_string(),
                "Safari".to_string(),
                "Close\tWindow".to_string(),
            ],
            vec![
                "cmd+q".to_string(),
                "Mail".to_string(),
                "Quit\nMail".to_string(),
            ],
        ];
        assert_eq!(
            tsv(&rows),
            "cmd+w\tSafari\tClose Window\ncmd+q\tMail\tQuit Mail\n"
        );
    }
}
