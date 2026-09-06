//! `tussle free` — every key with a given set of modifiers, and whether
//! it is free. One line per combo: `free` when nothing is bound to it,
//! `app-menu` when only app menu items use it (free as a global hotkey,
//! at the cost of those menu items), `taken` when a global binding owns
//! it.

use anyhow::{Context, Result};
use serde::Serialize;
use tussle_core::{Binding, HotkeyIndex, Key, KeyCombo, Modifiers, NamedKey, Scope};

use crate::cli::output::{emit_json, print_table, report_warnings};
use crate::cli::sources::{default_sources, warn_if_no_accessibility};

#[derive(Serialize)]
struct FreeJson {
    combo: String,
    status: &'static str,
    owners: Vec<String>,
}

/// Keys worth offering: every printable ANSI key plus the named keys that
/// commonly carry shortcuts.
fn candidate_keys() -> Vec<Key> {
    use NamedKey::*;
    let mut keys: Vec<Key> = ('a'..='z').map(Key::Char).collect();
    keys.extend(('0'..='9').map(Key::Char));
    keys.extend("`-=[]\\;',./".chars().map(Key::Char));
    keys.extend(
        [
            Space, Return, Tab, Escape, Up, Down, Left, Right, F1, F2, F3, F4, F5, F6, F7, F8, F9,
            F10, F11, F12,
        ]
        .into_iter()
        .map(Key::Named),
    );
    keys
}

pub fn free(
    modifiers_arg: &str,
    as_json: bool,
    ax_timeout: f32,
    ax_concurrency: usize,
    only: &[String],
) -> Result<()> {
    // Reuse the combo parser by appending a placeholder key.
    let modifiers: Modifiers = KeyCombo::parse(&format!("{modifiers_arg}+a"))
        .with_context(|| format!("parsing modifiers {modifiers_arg:?}"))?
        .modifiers;
    if modifiers.is_empty() {
        anyhow::bail!("give at least one modifier, e.g. `tussle free ctrl+opt`");
    }

    let sources = default_sources(ax_timeout, ax_concurrency, Vec::new(), only)?;
    warn_if_no_accessibility(&sources);
    let index = HotkeyIndex::scan(sources.iter().map(|s| s.as_ref()));
    report_warnings(&index);

    let rows: Vec<(KeyCombo, &'static str, Vec<&Binding>)> = candidate_keys()
        .into_iter()
        .map(|key| {
            let combo = KeyCombo { modifiers, key };
            let claimants = index.find(&combo);
            let status = if claimants.is_empty() {
                "free"
            } else if claimants.iter().all(|b| b.source.scope() == Scope::App) {
                "app-menu"
            } else {
                "taken"
            };
            (combo, status, claimants)
        })
        .collect();

    if as_json {
        let out: Vec<FreeJson> = rows
            .iter()
            .map(|(combo, status, bs)| FreeJson {
                combo: combo.to_string(),
                status,
                owners: owners(bs),
            })
            .collect();
        return emit_json(&out);
    }

    let table: Vec<Vec<String>> = rows
        .iter()
        .map(|(combo, status, bs)| {
            let owner = match *status {
                "taken" => bs
                    .iter()
                    .filter(|b| b.source.scope() == Scope::Global)
                    .map(|b| format!("{}: {}", b.source.owner(), b.label))
                    .collect::<Vec<_>>()
                    .join("; "),
                _ => owners(bs).join(", "),
            };
            vec![combo.to_string(), (*status).to_string(), owner]
        })
        .collect();
    print_table(&["Combo", "Status", "Owner"], &table);
    Ok(())
}

fn owners(bs: &[&Binding]) -> Vec<String> {
    let mut names: Vec<String> = bs.iter().map(|b| b.source.owner().to_string()).collect();
    names.sort();
    names.dedup();
    names
}
