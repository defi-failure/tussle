//! `tussle conflicts` — combos where bindings get in each other's way.

use anyhow::Result;
use serde::Serialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tussle_core::{Binding, Conflict, ConflictKind, HotkeyIndex, Winner};

use crate::cli::output::{BindingJson, VerdictJson, emit_json, layer_label, report_warnings};
use crate::cli::sources::{default_sources, warn_if_no_accessibility};

/// How many blocked bindings to name in the table before summarising.
const MAX_LISTED_LOSERS: usize = 4;

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

    let mut builder = Builder::default();
    builder.push_record(["Combo", "Kind", "Fires", "Blocked"]);
    for c in &found {
        builder.push_record([
            c.combo.to_string(),
            kind_label(c.kind).to_string(),
            fires(c),
            blocked(c),
        ]);
    }
    println!();
    println!("{}", builder.build().with(Style::psql()));
    Ok(())
}

fn kind_label(kind: ConflictKind) -> &'static str {
    match kind {
        ConflictKind::Contested => "contested",
        ConflictKind::Shadowed => "shadowed",
    }
}

fn name(b: &Binding) -> String {
    format!("{}: {}", b.source.owner(), b.label)
}

fn fires(c: &Conflict<'_>) -> String {
    match c.winner {
        Winner::Global(b) => name(b),
        Winner::Contested(layer) => format!("one of the {} bindings", layer_label(layer)),
        Winner::FrontmostApp | Winner::Nobody => String::new(),
    }
}

/// Everything that does not fire, shortened past `MAX_LISTED_LOSERS`.
fn blocked(c: &Conflict<'_>) -> String {
    let losers: Vec<String> = c
        .bindings
        .iter()
        .filter(|b| !matches!(c.winner, Winner::Global(w) if std::ptr::eq(w, **b)))
        .map(|b| name(b))
        .collect();
    if losers.len() <= MAX_LISTED_LOSERS {
        losers.join(", ")
    } else {
        format!(
            "{}, +{} more",
            losers[..MAX_LISTED_LOSERS].join(", "),
            losers.len() - MAX_LISTED_LOSERS
        )
    }
}
