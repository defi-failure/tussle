//! Serializable shapes for the `--json` output mode and the helper that
//! prints them. Table rendering stays inside each command so column choices
//! live next to the command that uses them.

use anyhow::Result;
use serde::Serialize;
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
