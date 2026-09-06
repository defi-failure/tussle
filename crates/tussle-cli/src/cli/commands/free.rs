//! `tussle free` — which combos with a given set of modifiers are unused.

use anyhow::{Context, Result};
use serde::Serialize;
use tussle_core::{Binding, HotkeyIndex, Key, KeyCombo, Modifiers, NamedKey, Scope};

use crate::cli::output::{emit_json, report_warnings};
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

    let mut free: Vec<KeyCombo> = Vec::new();
    let mut app_only: Vec<(KeyCombo, Vec<&Binding>)> = Vec::new();
    let mut taken: Vec<(KeyCombo, Vec<&Binding>)> = Vec::new();
    for key in candidate_keys() {
        let combo = KeyCombo { modifiers, key };
        let claimants = index.find(&combo);
        if claimants.is_empty() {
            free.push(combo);
        } else if claimants.iter().all(|b| b.source.scope() == Scope::App) {
            app_only.push((combo, claimants));
        } else {
            taken.push((combo, claimants));
        }
    }

    if as_json {
        let mut rows: Vec<FreeJson> = free
            .iter()
            .map(|c| FreeJson {
                combo: c.to_string(),
                status: "free",
                owners: vec![],
            })
            .collect();
        rows.extend(app_only.iter().map(|(c, bs)| FreeJson {
            combo: c.to_string(),
            status: "app_menus_only",
            owners: owners(bs),
        }));
        rows.extend(taken.iter().map(|(c, bs)| FreeJson {
            combo: c.to_string(),
            status: "taken",
            owners: owners(bs),
        }));
        return emit_json(&rows);
    }

    println!();
    println!(
        "Free with {modifiers} ({} of {}): nothing is bound to these anywhere.",
        free.len(),
        candidate_keys().len()
    );
    println!("  {}", wrap(free.iter().map(ToString::to_string), 76));
    if !app_only.is_empty() {
        println!();
        println!(
            "Used only inside app menus ({}): free as a global hotkey, but that app's menu \
             item stops working while the hotkey is active.",
            app_only.len()
        );
        println!(
            "  {}",
            wrap(
                app_only
                    .iter()
                    .map(|(c, bs)| format!("{c} ({})", owners(bs).join(", "))),
                76
            )
        );
    }
    if !taken.is_empty() {
        println!();
        println!("Taken globally ({}):", taken.len());
        for (c, bs) in &taken {
            println!("  {c:<18} {}", owners_with_labels(bs).join("; "));
        }
    }
    Ok(())
}

fn owners(bs: &[&Binding]) -> Vec<String> {
    let mut names: Vec<String> = bs.iter().map(|b| b.source.owner().to_string()).collect();
    names.sort();
    names.dedup();
    names
}

fn owners_with_labels(bs: &[&Binding]) -> Vec<String> {
    bs.iter()
        .filter(|b| b.source.scope() == Scope::Global)
        .map(|b| format!("{}: {}", b.source.owner(), b.label))
        .collect()
}

/// Join items with ", ", breaking lines at about `width` columns.
fn wrap(items: impl Iterator<Item = String>, width: usize) -> String {
    let mut out = String::new();
    let mut col = 0;
    for (i, item) in items.enumerate() {
        let sep = if i == 0 { "" } else { ", " };
        if col > 0 && col + sep.len() + item.chars().count() > width {
            out.push_str(",\n  ");
            col = 0;
        } else {
            out.push_str(sep);
            col += sep.len();
        }
        col += item.chars().count();
        out.push_str(&item);
    }
    out
}
