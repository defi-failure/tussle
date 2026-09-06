//! `tussle conflicts` — combos where bindings get in each other's way.
//!
//! For a person: one block per combo, its kind (`shadowed` or
//! `contested`), then the binding that `wins` and every binding that
//! `never fires`, one per line. For a program (piped, or `--plain`):
//! one tab-separated line per binding with combo, kind, role, layer,
//! owner and action. Nothing is printed when there are none.

use anyhow::Result;
use serde::Serialize;
use tussle_core::{Binding, Conflict, ConflictKind, HotkeyIndex, Winner};

use crate::cli::output::{
    BindingJson, VerdictJson, emit_json, human_output, layer_label, no_results, report_warnings,
    tsv,
};
use crate::cli::sources::{default_sources, warn_if_no_accessibility};

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
        no_results("no conflicts");
    } else if human_output() {
        let blocks: Vec<String> = found.iter().map(render).collect();
        print!("{}", blocks.join("\n"));
    } else {
        let mut rows = Vec::new();
        for c in &found {
            for b in &c.bindings {
                if let Some(role) = role(c, b) {
                    rows.push(vec![
                        c.combo.to_string(),
                        kind_label(c.kind).to_string(),
                        role.to_string(),
                        layer_label(b.source.layer()).to_string(),
                        b.source.owner().to_string(),
                        b.label.clone(),
                    ]);
                }
            }
        }
        print!("{}", tsv(&rows));
    }
    Ok(())
}

/// Role of `b` in conflict `c`; `None` for a binding owned by the winner,
/// which is the same function reachable twice rather than a loser.
fn role(c: &Conflict<'_>, b: &Binding) -> Option<&'static str> {
    Some(match c.winner {
        Winner::Global(w) if std::ptr::eq(w, b) => "wins",
        Winner::Global(w) if w.source.owner() == b.source.owner() => return None,
        Winner::Global(_) => "never fires",
        Winner::Contested(layer) if b.source.layer() == layer => "contested",
        Winner::Contested(_) => "never fires",
        Winner::FrontmostApp | Winner::Nobody => "app",
    })
}

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
    let mut out = format!("{}  {}\n", c.combo, kind_label(c.kind));
    match c.winner {
        Winner::Global(w) => {
            out.push_str(&format!("  wins         {}\n", describe(w)));
            // Bindings owned by the winner are the same function reachable
            // twice, not losers.
            let losers = c
                .bindings
                .iter()
                .copied()
                .filter(|b| !std::ptr::eq(*b, w) && b.source.owner() != w.source.owner());
            push_lines(&mut out, "never fires", losers);
        }
        Winner::Contested(layer) => {
            let (contenders, rest): (Vec<&Binding>, Vec<&Binding>) = c
                .bindings
                .iter()
                .copied()
                .partition(|b| b.source.layer() == layer);
            push_lines(&mut out, "contested", contenders.into_iter());
            push_lines(&mut out, "never fires", rest.into_iter());
        }
        Winner::FrontmostApp | Winner::Nobody => {
            push_lines(&mut out, "bindings", c.bindings.iter().copied());
        }
    }
    out
}

fn push_lines<'a>(out: &mut String, heading: &str, items: impl Iterator<Item = &'a Binding>) {
    for (i, b) in items.enumerate() {
        let head = if i == 0 { heading } else { "" };
        out.push_str(&format!("  {head:<12} {}\n", describe(b)));
    }
}
