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
        print!("{}", describe_probe(&combo, probe, &matches));
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

/// What happened in the `PROBE_SETTLE` window after the key went through,
/// cross-checked against the bindings the static sources know about.
///
/// A reaction from an app that no source lists means the app reacts to
/// the key from its own code; and when a system shortcut fired *and*
/// such an app reacted, the app is observing the key rather than
/// claiming it, which is why both happen and why no layer order can
/// stop it.
fn describe_probe(combo: &KeyCombo, probe: &Probe, matches: &[&Binding]) -> String {
    let mut out = format!(
        "Observed within {} ms of letting {combo} through:\n",
        PROBE_SETTLE.as_millis()
    );
    let system_fired = if let Some((before, after)) = &probe.input_source_change {
        let by = matches
            .iter()
            .find(|b| matches!(b.source, BindingSource::SystemSymbolicHotkey { .. }))
            .map(|b| format!("  ({}: {})", b.source.owner(), b.label))
            .unwrap_or_default();
        out.push_str(&format!(
            "  input source changed: {before} -> {after}{by}\n"
        ));
        true
    } else {
        false
    };
    let mut unexplained: Vec<String> = Vec::new();
    for r in &probe.reactions {
        let known = is_known(r, matches);
        out.push_str(&format!("  {}", describe_reaction(r)));
        if known {
            out.push('\n');
        } else {
            let who = reactor_name(r);
            out.push_str("  (in no source)\n");
            unexplained.push(who);
        }
    }
    if probe.reactions.is_empty() && probe.input_source_change.is_none() {
        out.push_str("  nothing came to the front, no window opened, input source unchanged\n");
    }
    for who in &unexplained {
        out.push_str(&format!(
            "{who} reacts to {combo} from its own code: no file or menu records it, so tussle \
             cannot list it. To change it: {who}'s own settings.\n"
        ));
    }
    if system_fired && !unexplained.is_empty() {
        out.push_str(
            "Both fired: the system shortcut only blocks apps that claim the key through \
             their menus; an app that merely watches keystrokes still sees it. Turn off \
             whichever you do not want.\n",
        );
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

fn describe_reaction(r: &Reaction) -> String {
    let who = reactor_name(r);
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

#[cfg(test)]
mod tests {
    use super::*;
    use tussle_core::{Key, Modifiers, SystemDispatch};

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
