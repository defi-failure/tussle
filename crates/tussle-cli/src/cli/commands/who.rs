//! `tussle who` — every binding that claims a combo, in the order they
//! would see the key.
//!
//! Output is one line per claimant. The `Fires` column carries the
//! verdict: `yes` for the global binding that gets the key, `contested`
//! when several share the first layer, `app` when only app menus claim
//! the combo (the frontmost app handles it), `off` for a disabled
//! binding, and blank for a binding that never sees the key. When nothing
//! is bound, stdout stays empty and a person gets a note on stderr.

use anyhow::{Context, Result, bail};
use serde::Serialize;
use tussle_core::capture::{self, Captured, Probe, Reaction, ReactionKind};
use tussle_core::{Binding, HotkeyIndex, KeyCombo, Winner};

use crate::cli::output::{
    BindingJson, VerdictJson, emit_json, how_to_change, layer_label, no_results, print_table,
    report_warnings,
};
use crate::cli::sources::{default_sources, warn_if_no_accessibility};

#[derive(Serialize)]
struct WhoJson<'a> {
    combo: String,
    verdict: VerdictJson<'a>,
    /// Where to change the binding that fires, when one does.
    #[serde(skip_serializing_if = "Option::is_none")]
    change: Option<String>,
    bindings: Vec<BindingJson<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed: Option<ObservedJson>,
}

/// What `--probe` saw after letting the key through.
#[derive(Serialize)]
struct ObservedJson {
    settle_ms: u64,
    reactions: Vec<ReactionJson>,
    input_source_change: Option<(String, String)>,
}

#[derive(Serialize)]
struct ReactionJson {
    pid: i32,
    app_name: Option<String>,
    bundle_id: Option<String>,
    kind: &'static str,
    new_windows: usize,
    /// Whether some listed binding belongs to the reacting app. `false`
    /// means the app reacts from its own code and no source records it.
    known: bool,
}

/// How long `--probe` watches for reactions after the key.
const PROBE_SETTLE: std::time::Duration = std::time::Duration::from_millis(800);

pub fn who(
    combo_arg: Option<String>,
    as_json: bool,
    probe: bool,
    explain: bool,
    ax_timeout: f32,
    ax_concurrency: usize,
    only: &[String],
) -> Result<()> {
    if probe && combo_arg.is_some() {
        bail!("--probe watches a key you press; leave the combo argument out");
    }
    let mut observed: Option<Probe> = None;
    let combo = match combo_arg {
        Some(text) => KeyCombo::parse(&text).with_context(|| format!("parsing combo {text:?}"))?,
        None => {
            let (captured, probed) = capture_interactively(probe)?;
            observed = probed;
            match captured {
                Some(c) => c,
                None => return Ok(()),
            }
        }
    };

    let sources = default_sources(ax_timeout, ax_concurrency, Vec::new(), only)?;
    warn_if_no_accessibility(&sources);
    let index = HotkeyIndex::scan(sources.iter().map(|s| s.as_ref()));
    report_warnings(&index);

    let matches = index.find(&combo);
    let winner = index.winner(&combo);
    let off: Vec<&Binding> = index
        .iter()
        .filter(|b| !b.enabled && b.combo == combo)
        .collect();
    tracing::info!(
        combo = %combo,
        matches = matches.len(),
        disabled = off.len(),
        "lookup complete",
    );
    let change = match winner {
        Winner::Global(w) => Some(how_to_change(w)),
        _ => None,
    };

    if as_json {
        return emit_json(&WhoJson {
            combo: combo.to_string(),
            verdict: VerdictJson::from(winner),
            change,
            bindings: matches
                .iter()
                .chain(off.iter())
                .map(|b| BindingJson::from(*b))
                .collect(),
            observed: observed.as_ref().map(|p| observed_json(p, &matches)),
        });
    }

    if matches.is_empty() && off.is_empty() {
        no_results(&format!("nothing bound to {combo}"));
        return Ok(());
    }

    let row = |fires: &str, b: &Binding| {
        vec![
            fires.to_string(),
            layer_label(b.source.layer()).to_string(),
            b.source.owner().to_string(),
            b.label.clone(),
        ]
    };
    let rows: Vec<Vec<String>> = matches
        .iter()
        .map(|b| row(fires(winner, b), b))
        .chain(off.iter().map(|b| row("off", b)))
        .collect();
    print_table(&["Fires", "Layer", "Owner", "Action"], &rows);

    if explain && let Some(change) = &change {
        println!("change: {change}");
    }
    if let Some(p) = &observed {
        print!("{}", observed_lines(p, &matches, explain));
    }
    Ok(())
}

/// The `Fires` cell for one enabled claimant.
fn fires(winner: Winner<'_>, b: &Binding) -> &'static str {
    match winner {
        Winner::Global(w) if std::ptr::eq(w, b) => "yes",
        Winner::Contested(layer) if b.source.layer() == layer => "contested",
        Winner::FrontmostApp => "app",
        _ => "",
    }
}

fn observed_json(p: &Probe, matches: &[&Binding]) -> ObservedJson {
    ObservedJson {
        settle_ms: PROBE_SETTLE.as_millis() as u64,
        reactions: p
            .reactions
            .iter()
            .map(|r| ReactionJson {
                pid: r.pid,
                app_name: r.app_name.clone(),
                bundle_id: r.bundle_id.clone(),
                kind: match r.kind {
                    ReactionKind::Activated => "activated",
                    ReactionKind::NewWindows(_) => "new_windows",
                },
                new_windows: match r.kind {
                    ReactionKind::NewWindows(n) => n,
                    ReactionKind::Activated => 0,
                },
                known: is_known(r, matches),
            })
            .collect(),
        input_source_change: p.input_source_change.clone(),
    }
}

/// One `observed` line per thing that happened after the key went
/// through. A reaction from an app no source lists is marked
/// `in no source`: the app reacts from its own code.
fn observed_lines(p: &Probe, matches: &[&Binding], explain: bool) -> String {
    let mut out = String::new();
    if let Some((before, after)) = &p.input_source_change {
        out.push_str(&format!("observed  input source  {before} -> {after}\n"));
    }
    let mut unexplained: Vec<String> = Vec::new();
    for r in &p.reactions {
        let who = reactor_name(r);
        let what = match r.kind {
            ReactionKind::Activated => "activated".to_string(),
            ReactionKind::NewWindows(n) => {
                format!("{n} new window{}", if n == 1 { "" } else { "s" })
            }
        };
        let note = if is_known(r, matches) {
            ""
        } else {
            unexplained.push(who.clone());
            "  in no source"
        };
        out.push_str(&format!("observed  {what:<13} {who}{note}\n"));
    }
    if p.reactions.is_empty() && p.input_source_change.is_none() {
        out.push_str("observed  nothing\n");
    }
    if explain {
        for who in &unexplained {
            out.push_str(&format!(
                "change: {who}'s own settings ({who} reacts to the key from its own code)\n"
            ));
        }
    }
    out
}

/// Whether some static binding on this combo belongs to the reacting app.
fn is_known(r: &Reaction, matches: &[&Binding]) -> bool {
    matches.iter().any(|b| {
        let same_bundle = match (b.source.bundle_id(), r.bundle_id.as_deref()) {
            (Some(a), Some(c)) => a.eq_ignore_ascii_case(c),
            _ => false,
        };
        let same_name = r
            .app_name
            .as_deref()
            .is_some_and(|n| n.eq_ignore_ascii_case(b.source.owner()));
        same_bundle || same_name
    })
}

fn reactor_name(r: &Reaction) -> String {
    r.app_name
        .clone()
        .or_else(|| r.bundle_id.clone())
        .unwrap_or_else(|| format!("pid {}", r.pid))
}

/// Run the interactive capture flow. Returns:
///   - `Ok((Some(combo), probe))` for a normal hotkey to look up,
///   - `Ok((None, _))` when the user pressed a macOS system action — we
///     already printed the explanation and the caller should bail out,
///   - `Err(_)` on capture failure (no Input Monitoring permission, etc.).
fn capture_interactively(probe: bool) -> Result<(Option<KeyCombo>, Option<Probe>)> {
    if probe {
        eprintln!("Press the hotkey to look up; it will go through (Ctrl+C to abort)...");
    } else {
        eprintln!("Press the hotkey to look up (Ctrl+C to abort)...");
    }
    let feedback = |mods: tussle_core::Modifiers| {
        use std::io::Write;
        let mut stderr = std::io::stderr().lock();
        // \x1B[2K clears the entire line; \r returns the cursor.
        if mods.is_empty() {
            let _ = write!(stderr, "\r\x1B[2K");
        } else {
            let _ = write!(stderr, "\r\x1B[2KHolding: {mods}+");
        }
        let _ = stderr.flush();
    };
    let (captured, probed) = if probe {
        let p = capture::capture_and_probe(feedback, PROBE_SETTLE).context("probing keystroke")?;
        (p.captured, Some(p))
    } else {
        (
            capture::capture_one(feedback).context("capturing keystroke")?,
            None,
        )
    };

    let combo = match captured {
        Captured::Combo(c) => {
            eprintln!("\r\x1B[2KCaptured: {c}");
            Some(c)
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
            None
        }
    };
    Ok((combo, probed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tussle_core::{BindingSource, Key, Modifiers, SystemDispatch};

    fn reaction(name: &str, bundle: Option<&str>) -> Reaction {
        Reaction {
            pid: 1,
            app_name: Some(name.into()),
            bundle_id: bundle.map(Into::into),
            kind: ReactionKind::NewWindows(1),
        }
    }

    #[test]
    fn reactions_are_matched_to_claimants_by_bundle_or_name() {
        let combo = KeyCombo {
            modifiers: Modifiers::CTRL,
            key: Key::Named(tussle_core::NamedKey::Space),
        };
        let system = Binding {
            combo,
            source: BindingSource::SystemSymbolicHotkey {
                id: Some(60),
                dispatch: SystemDispatch::BeforeApps,
            },
            label: "Select the previous input source".into(),
            enabled: true,
        };
        let warp = Binding {
            combo,
            source: BindingSource::AppMenuItem {
                bundle_id: "dev.warp.Warp-Stable".into(),
                app_name: Some("Warp".into()),
                menu_path: vec![],
            },
            label: "New Agent Pane".into(),
            enabled: true,
        };
        let matches = [&system, &warp];
        assert!(is_known(&reaction("Warp", None), &matches));
        assert!(is_known(
            &reaction("warp", Some("dev.warp.Warp-Stable")),
            &matches
        ));
        assert!(!is_known(
            &reaction("Codex", Some("com.openai.codex")),
            &matches
        ));
    }
}
