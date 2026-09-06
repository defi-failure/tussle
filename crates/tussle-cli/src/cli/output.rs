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
