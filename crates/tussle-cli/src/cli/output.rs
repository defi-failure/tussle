//! Serializable shapes for the `--json` output mode and the helper that
//! prints them. Table rendering stays inside each command so column choices
//! live next to the command that uses them.

use anyhow::Result;
use serde::Serialize;
use tussle_core::{Binding, BindingSource};

#[derive(Serialize)]
pub(super) struct BindingJson<'a> {
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

/// Print `bindings` as pretty-printed JSON to stdout.
pub(super) fn emit_json(bindings: &[Binding]) -> Result<()> {
    let rows: Vec<BindingJson> = bindings.iter().map(BindingJson::from).collect();
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}
