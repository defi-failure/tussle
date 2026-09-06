//! `tussle doctor` — are the permissions and sources in place for a
//! complete answer?

use anyhow::Result;
use serde::Serialize;
use tussle_core::capture::{self, PermissionStatus};
use tussle_core::sources::accessibility;
use tussle_core::sources::symbolichotkeys::{LiveTable, SymbolicHotkeys};
use tussle_core::{HotkeyIndex, Source};

use crate::cli::output::emit_json;
use crate::cli::sources::default_sources;

#[derive(Serialize)]
struct CheckJson {
    check: &'static str,
    status: String,
    detail: String,
}

pub fn doctor(as_json: bool, ax_timeout: f32, ax_concurrency: usize) -> Result<()> {
    let mut checks: Vec<CheckJson> = Vec::new();

    let ax = accessibility::is_trusted();
    checks.push(CheckJson {
        check: "Accessibility",
        status: if ax { "granted" } else { "missing" }.into(),
        detail: if ax {
            "menu shortcuts of running apps can be read".into()
        } else {
            "without it every app's menu shortcuts are invisible: System Settings → Privacy & \
             Security → Accessibility, add your terminal"
                .into()
        },
    });

    let im = capture::input_monitoring_status();
    checks.push(CheckJson {
        check: "Input Monitoring",
        status: match im {
            PermissionStatus::Granted => "granted",
            PermissionStatus::Denied => "denied",
            PermissionStatus::Undetermined => "not asked yet",
        }
        .into(),
        detail: match im {
            PermissionStatus::Granted => "`who` can capture a pressed key".into(),
            PermissionStatus::Denied => "interactive `who` cannot capture keys: System Settings → \
                                        Privacy & Security → Input Monitoring, add your terminal"
                .into(),
            PermissionStatus::Undetermined => {
                "only needed for interactive `who`; macOS asks the first time".into()
            }
        },
    });

    let prefs = dirs::preference_dir();
    let plist = prefs
        .as_ref()
        .map(|p| p.join("com.apple.symbolichotkeys.plist"));
    let live = SymbolicHotkeys::new(plist.clone().unwrap_or_default())
        .with_live_table(LiveTable::System)
        .scan();
    checks.push(CheckJson {
        check: "System shortcuts",
        status: match &live {
            Ok(scan) if scan.warnings.is_empty() => "ok".into(),
            Ok(_) => "partial".into(),
            Err(_) => "failed".into(),
        },
        detail: match &live {
            Ok(scan) => format!(
                "{} entries from the live table{}",
                scan.bindings.len(),
                scan.warnings
                    .first()
                    .map(|w| format!("; {w}"))
                    .unwrap_or_default()
            ),
            Err(e) => e.to_string(),
        },
    });

    checks.push(CheckJson {
        check: "Preferences",
        status: match &plist {
            Some(p) if p.exists() => "ok".into(),
            Some(_) => "missing".into(),
            None => "unknown".into(),
        },
        detail: plist
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "could not locate ~/Library/Preferences".into()),
    });

    if ax {
        let sources = default_sources(ax_timeout, ax_concurrency, Vec::new(), &[])?;
        let index = HotkeyIndex::scan(sources.iter().map(|s| s.as_ref()));
        let unresponsive = index
            .warnings()
            .iter()
            .filter(|w| matches!(w, tussle_core::ScanWarning::Unresponsive { .. }))
            .count();
        let apps: std::collections::HashSet<&str> = index
            .enabled()
            .filter_map(|b| b.source.bundle_id())
            .collect();
        checks.push(CheckJson {
            check: "Running apps",
            status: if unresponsive == 0 { "ok" } else { "partial" }.into(),
            detail: format!(
                "{} apps with shortcuts, {} bindings, {} unresponsive",
                apps.len(),
                index.len(),
                unresponsive
            ),
        });
    }

    if as_json {
        return emit_json(&checks);
    }
    println!();
    for c in &checks {
        println!("{:<18} {:<14} {}", c.check, c.status, c.detail);
    }
    Ok(())
}
