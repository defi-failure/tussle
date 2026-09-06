//! `tussle conflicts` — combos where bindings get in each other's way.

use anyhow::Result;
use serde::Serialize;
use tussle_core::{Binding, Conflict, ConflictKind, HotkeyIndex, Winner};

use crate::cli::output::{BindingJson, VerdictJson, emit_json, layer_label, report_warnings};
use crate::cli::sources::{default_sources, warn_if_no_accessibility};

/// How many losing bindings to list per conflict before summarising.
const MAX_LISTED_LOSERS: usize = 6;

#[derive(Serialize)]
struct ConflictJson<'a> {
    combo: String,
    kind: &'static str,
    verdict: VerdictJson<'a>,
    bindings: Vec<BindingJson<'a>>,
}

pub fn conflicts(
    as_json: bool,
    ax_timeout: f32,
    ax_concurrency: usize,
    only: &[String],
) -> Result<()> {
    let sources = default_sources(ax_timeout, ax_concurrency, Vec::new(), only)?;
    warn_if_no_accessibility(&sources);
    let index = HotkeyIndex::scan(sources.iter().map(|s| s.as_ref()));
    report_warnings(&index);

    let found = index.conflicts();
    tracing::info!(conflicts = found.len(), "conflict scan complete");

    if as_json {
        let rows: Vec<ConflictJson> = found
            .iter()
            .map(|c| ConflictJson {
                combo: c.combo.to_string(),
                kind: kind_label(c.kind),
                verdict: VerdictJson::from(c.winner),
                bindings: c.bindings.iter().map(|b| BindingJson::from(*b)).collect(),
            })
            .collect();
        return emit_json(&rows);
    }

    if found.is_empty() {
        println!("(no conflicts found)");
        return Ok(());
    }

    // One block per conflict: a table cannot hold forty losers on a line.
    println!();
    for c in &found {
        println!("{}", render(c));
    }
    println!(
        "{} conflict{}. \"wins\" gets the key; \"never fires\" no longer reaches its owner.",
        found.len(),
        if found.len() == 1 { "" } else { "s" }
    );
    Ok(())
}

/// Stable identifier used in JSON.
fn kind_label(kind: ConflictKind) -> &'static str {
    match kind {
        ConflictKind::Contested => "contested",
        ConflictKind::Shadowed => "shadowed",
    }
}

fn describe(b: &Binding) -> String {
    format!(
        "{}: {}  [{}]",
        b.source.owner(),
        b.label,
        layer_label(b.source.layer())
    )
}

fn render(c: &Conflict<'_>) -> String {
    let mut out = String::new();
    match c.winner {
        Winner::Global(w) => {
            out.push_str(&format!("{}  {}\n", c.combo, "global beats app menu"));
            out.push_str(&format!("  wins         {}\n", describe(w)));
            let losers: Vec<&Binding> = c
                .bindings
                .iter()
                .copied()
                .filter(|b| !std::ptr::eq(*b, w) && b.source.owner() != w.source.owner())
                .collect();
            push_list(&mut out, "never fires", &losers);
        }
        Winner::Contested(layer) => {
            out.push_str(&format!(
                "{}  global vs global on the {} layer; which one fires cannot be told from outside\n",
                c.combo,
                layer_label(layer)
            ));
            let (contenders, rest): (Vec<&Binding>, Vec<&Binding>) = c
                .bindings
                .iter()
                .copied()
                .partition(|b| b.source.layer() == layer);
            push_list(&mut out, "contested", &contenders);
            push_list(&mut out, "never fires", &rest);
        }
        Winner::FrontmostApp | Winner::Nobody => {
            out.push_str(&format!("{}\n", c.combo));
            push_list(&mut out, "bindings", &c.bindings);
        }
    }
    out
}

fn push_list(out: &mut String, heading: &str, items: &[&Binding]) {
    for (i, b) in items.iter().take(MAX_LISTED_LOSERS).enumerate() {
        let head = if i == 0 { heading } else { "" };
        out.push_str(&format!("  {head:<12} {}\n", describe(b)));
    }
    if items.len() > MAX_LISTED_LOSERS {
        out.push_str(&format!(
            "  {:<12} … and {} more\n",
            "",
            items.len() - MAX_LISTED_LOSERS
        ));
    }
}
