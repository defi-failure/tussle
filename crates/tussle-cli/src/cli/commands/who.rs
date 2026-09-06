//! `tussle who` — for a given combo (parsed or interactively captured),
//! list every binding that claims it, in the order they would see the
//! key, and say which one fires.

use anyhow::{Context, Result, bail};
use serde::Serialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tussle_core::capture::{self, Captured, Probe, Reaction, ReactionKind};
use tussle_core::{Binding, BindingSource, HotkeyIndex, KeyCombo, Winner};

use crate::cli::output::{
    BindingJson, VerdictJson, emit_json, how_to_change, layer_label, report_warnings,
};
use crate::cli::sources::{default_sources, warn_if_no_accessibility};

#[derive(Serialize)]
struct WhoJson<'a> {
    combo: String,
    verdict: VerdictJson<'a>,
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
}

/// How long `--probe` watches for reactions after the key.
const PROBE_SETTLE: std::time::Duration = std::time::Duration::from_millis(800);

pub fn who(
    combo_arg: Option<String>,
    as_json: bool,
    probe: bool,
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
    // Switched-off claimants explain a free combo ("⌃1 would switch
    // desktops, but that shortcut is off"), so list them after the live
    // ones rather than hiding them.
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

    if as_json {
        return emit_json(&WhoJson {
            combo: combo.to_string(),
            verdict: VerdictJson::from(winner),
            bindings: matches
                .iter()
                .chain(off.iter())
                .map(|b| BindingJson::from(*b))
                .collect(),
            observed: observed.as_ref().map(observed_json),
        });
    }

    if matches.is_empty() && off.is_empty() {
        println!("nothing bound to {combo}");
        return Ok(());
    }

    let mut builder = Builder::default();
    builder.push_record(["Fires", "Layer", "Owner", "Action"]);
    for b in &matches {
        let fires = match winner {
            Winner::Global(w) if std::ptr::eq(w, *b) => "yes",
            _ => "",
        };
        builder.push_record([
            fires,
            layer_label(b.source.layer()),
            b.source.owner(),
            b.label.as_str(),
        ]);
    }
    for b in &off {
        builder.push_record([
            "off",
            layer_label(b.source.layer()),
            b.source.owner(),
            b.label.as_str(),
        ]);
    }
    println!();
    println!("{}", builder.build().with(Style::psql()));
    println!();
    println!("{}", describe(&combo, winner, &matches));
    if let Winner::Global(w) = winner {
        println!("To change it: {}.", how_to_change(w));
    }
    if let Some(probe) = &observed {
        println!();
        print!("{}", describe_probe(&combo, probe));
    }
    Ok(())
}

fn observed_json(p: &Probe) -> ObservedJson {
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
            })
            .collect(),
        input_source_change: p.input_source_change.clone(),
    }
}

/// What happened in the `PROBE_SETTLE` window after the key went through.
fn describe_probe(combo: &KeyCombo, probe: &Probe) -> String {
    let mut out = format!(
        "Observed within {} ms of letting {combo} through:\n",
        PROBE_SETTLE.as_millis()
    );
    if let Some((before, after)) = &probe.input_source_change {
        out.push_str(&format!("  input source changed: {before} -> {after}\n"));
    }
    for r in &probe.reactions {
        out.push_str(&format!("  {}\n", describe_reaction(r)));
    }
    if probe.reactions.is_empty() && probe.input_source_change.is_none() {
        out.push_str("  nothing came to the front, no window opened, input source unchanged\n");
    }
    out
}

fn describe_reaction(r: &Reaction) -> String {
    let who = r
        .app_name
        .clone()
        .or_else(|| r.bundle_id.clone())
        .unwrap_or_else(|| format!("pid {}", r.pid));
    match r.kind {
        ReactionKind::Activated => format!("{who} came to the front"),
        ReactionKind::NewWindows(1) => format!("{who} opened a window"),
        ReactionKind::NewWindows(n) => format!("{who} opened {n} windows"),
    }
}

/// One sentence on who gets the key.
fn describe(combo: &KeyCombo, winner: Winner<'_>, matches: &[&Binding]) -> String {
    let count = matches.len();
    let all_apple_menu = !matches.is_empty()
        && matches
            .iter()
            .all(|b| matches!(b.source, BindingSource::AppleMenuItem { .. }));
    match winner {
        Winner::FrontmostApp if all_apple_menu => format!(
            "{combo} is an Apple menu item ({}), available in every app",
            matches[0].label
        ),
        Winner::Nobody => format!("nothing enabled is bound to {combo}"),
        Winner::Global(b) => {
            let losers = count - 1;
            let tail = match losers {
                0 => String::new(),
                1 => "; the other binding never sees this key".to_string(),
                n => format!("; the other {n} bindings never see this key"),
            };
            format!(
                "{combo} fires {} ({} layer): {}{tail}",
                b.source.owner(),
                layer_label(b.source.layer()),
                b.label
            )
        }
        Winner::Contested(layer) => format!(
            "{combo} is contested on the {} layer: which binding fires depends on \
             registration order, which cannot be observed from outside",
            layer_label(layer)
        ),
        Winner::FrontmostApp => {
            format!("{combo} is only bound in app menus: whichever app is frontmost handles it")
        }
    }
}

/// Run the interactive capture flow. Returns:
///   - `Ok(Some(combo))` for a normal hotkey to look up,
///   - `Ok(None)` when the user pressed a macOS system action — we already
///     printed the explanation and the caller should bail out cleanly,
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
            eprintln!("\r\x1B[2KCaptured: {c} — looking up...");
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
