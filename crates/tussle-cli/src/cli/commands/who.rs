//! `tussle who` — for a given combo (parsed or interactively captured),
//! list every binding that claims it, in the order they would see the
//! key, and say which one fires.

use anyhow::{Context, Result};
use serde::Serialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tussle_core::capture::{self, Captured};
use tussle_core::{HotkeyIndex, KeyCombo, Winner};

use crate::cli::output::{BindingJson, VerdictJson, emit_json, layer_label, report_warnings};
use crate::cli::sources::{default_sources, warn_if_no_accessibility};

#[derive(Serialize)]
struct WhoJson<'a> {
    combo: String,
    verdict: VerdictJson<'a>,
    bindings: Vec<BindingJson<'a>>,
}

pub fn who(
    combo_arg: Option<String>,
    as_json: bool,
    ax_timeout: f32,
    ax_concurrency: usize,
) -> Result<()> {
    let combo = match combo_arg {
        Some(text) => KeyCombo::parse(&text).with_context(|| format!("parsing combo {text:?}"))?,
        None => match capture_interactively()? {
            Some(c) => c,
            None => return Ok(()),
        },
    };

    let sources = default_sources(ax_timeout, ax_concurrency, Vec::new())?;
    warn_if_no_accessibility();
    let index = HotkeyIndex::scan(sources.iter().map(|s| s.as_ref()));
    report_warnings(&index);

    let matches = index.find(&combo);
    let winner = index.winner(&combo);
    tracing::info!(
        combo = %combo,
        matches = matches.len(),
        "lookup complete",
    );

    if as_json {
        return emit_json(&WhoJson {
            combo: combo.to_string(),
            verdict: VerdictJson::from(winner),
            bindings: matches.iter().map(|b| BindingJson::from(*b)).collect(),
        });
    }

    if matches.is_empty() {
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
    println!();
    println!("{}", builder.build().with(Style::psql()));
    println!();
    println!("{}", describe(&combo, winner, matches.len()));
    Ok(())
}

/// One sentence on who gets the key.
fn describe(combo: &KeyCombo, winner: Winner<'_>, count: usize) -> String {
    match winner {
        Winner::Nobody => format!("nothing bound to {combo}"),
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
fn capture_interactively() -> Result<Option<KeyCombo>> {
    eprintln!("Press the hotkey to look up (Ctrl+C to abort)...");
    let captured = capture::capture_one(|mods| {
        use std::io::Write;
        let mut stderr = std::io::stderr().lock();
        // \x1B[2K clears the entire line; \r returns the cursor.
        if mods.is_empty() {
            let _ = write!(stderr, "\r\x1B[2K");
        } else {
            let _ = write!(stderr, "\r\x1B[2KHolding: {mods}+");
        }
        let _ = stderr.flush();
    })
    .context("capturing keystroke")?;

    Ok(match captured {
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
    })
}
