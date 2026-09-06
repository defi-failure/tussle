//! macOS implementation of the Accessibility-API menu enumerator.

mod ax;
mod menu_walker;
mod modifiers;
mod running_apps;

use std::collections::HashSet;
use std::thread;

use crate::sources::SourceScan;
use crate::{Binding, BindingSource, KeyCombo, ScanError, ScanWarning};

use menu_walker::WalkResult;
use running_apps::RunningApp;

/// Multiplier applied to the messaging timeout when an app is asked a
/// second time. Apps woken from App Nap routinely need a few seconds for
/// their first Accessibility reply and answer quickly after that; with a
/// 1s first pass this gives them 5s.
const RETRY_TIMEOUT_FACTOR: f32 = 5.0;

pub(super) fn scan(
    messaging_timeout: f32,
    max_concurrency: usize,
    bundle_filter: &[String],
) -> Result<SourceScan, ScanError> {
    if !is_trusted() {
        tracing::warn!("Accessibility permission missing — skipping menu enumeration");
        return Ok(SourceScan::default());
    }

    // Each app's walk is a sequence of synchronous AX IPC calls — wallclock
    // is dominated by waiting for the target app's main thread to respond,
    // not by CPU. Spawning one OS thread per app lets the waits overlap;
    // since the threads are sleeping (not running), tens of concurrent
    // threads cost basically nothing. A bounded pool (rayon, threadpool)
    // would cap us at CPU-core count and serialize the rest, defeating the
    // point.
    //
    // We still apply a soft cap (`max_concurrency`) by walking apps in
    // chunks: it's a defensive bound for pathological sessions with
    // hundreds of running processes, not a performance lever for the
    // typical case.
    let mut apps = running_apps::list_running_apps();
    if !bundle_filter.is_empty() {
        let filter_lc: Vec<String> = bundle_filter.iter().map(|s| s.to_lowercase()).collect();
        let before = apps.len();
        apps.retain(|a| {
            matches_bundle_filter(a.bundle_id.as_deref(), a.app_name.as_deref(), &filter_lc)
        });
        tracing::debug!(
            kept = apps.len(),
            dropped = before - apps.len(),
            "applied bundle filter",
        );
    }
    let chunk_size = if max_concurrency == 0 {
        apps.len().max(1)
    } else {
        max_concurrency
    };

    let refs: Vec<&RunningApp> = apps.iter().collect();
    let mut results = walk_all(&refs, messaging_timeout, chunk_size);

    // Second pass for apps that did not answer in time. The first pass
    // deliberately uses a short timeout so one stuck app cannot stall the
    // scan; but an app that was merely asleep answers on the retry, and
    // dropping it would silently hide every one of its shortcuts.
    let slow: Vec<usize> = results
        .iter()
        .enumerate()
        .filter(|(_, r)| r.timed_out)
        .map(|(i, _)| i)
        .collect();
    let mut warnings = Vec::new();
    if !slow.is_empty() {
        let retry_timeout = retry_timeout(messaging_timeout);
        tracing::info!(
            apps = slow.len(),
            timeout_secs = retry_timeout,
            "retrying apps that did not answer in time",
        );
        let slow_refs: Vec<&RunningApp> = slow.iter().map(|&i| &apps[i]).collect();
        let retried = walk_all(&slow_refs, retry_timeout, chunk_size);
        for (&i, second) in slow.iter().zip(retried) {
            let first = std::mem::take(&mut results[i]);
            let (best, unresponsive) = resolve_retry(first, second);
            if unresponsive {
                warnings.push(ScanWarning::Unresponsive {
                    app: display_name(&apps[i]),
                });
            }
            results[i] = best;
        }
    }

    Ok(SourceScan {
        bindings: dedupe_apple_menu(results.into_iter().flat_map(|r| r.bindings)),
        warnings,
    })
}

/// Every app shows the same Apple menu, so its items come back once per
/// running app. Keep the first of each (combo, label) and pass everything
/// else through untouched.
fn dedupe_apple_menu(bindings: impl IntoIterator<Item = Binding>) -> Vec<Binding> {
    let mut seen: HashSet<(KeyCombo, String)> = HashSet::new();
    bindings
        .into_iter()
        .filter(|b| {
            !matches!(b.source, BindingSource::AppleMenuItem { .. })
                || seen.insert((b.combo, b.label.clone()))
        })
        .collect()
}

/// Walk every app, `chunk_size` at a time, one thread per app. Results
/// are in the same order as `apps`.
fn walk_all(apps: &[&RunningApp], timeout: f32, chunk_size: usize) -> Vec<WalkResult> {
    let mut results = Vec::with_capacity(apps.len());
    for batch in apps.chunks(chunk_size) {
        let batch_results: Vec<WalkResult> = thread::scope(|s| {
            let handles: Vec<_> = batch
                .iter()
                .map(|app| s.spawn(move || menu_walker::walk_app_menus(app, timeout)))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap_or_default())
                .collect()
        });
        results.extend(batch_results);
    }
    results
}

/// Timeout for the second attempt. `0` means "macOS default" and stays
/// `0`: there is nothing longer to offer.
fn retry_timeout(messaging_timeout: f32) -> f32 {
    if messaging_timeout > 0.0 {
        messaging_timeout * RETRY_TIMEOUT_FACTOR
    } else {
        0.0
    }
}

/// Pick the better of two walks of the same app. A finished second walk
/// wins outright. If the app timed out again, keep whichever attempt saw
/// more and report the app as unresponsive.
fn resolve_retry(first: WalkResult, second: WalkResult) -> (WalkResult, bool) {
    if !second.timed_out {
        return (second, false);
    }
    let best = if second.bindings.len() >= first.bindings.len() {
        second
    } else {
        first
    };
    (best, true)
}

fn display_name(app: &RunningApp) -> String {
    app.app_name
        .clone()
        .or_else(|| app.bundle_id.clone())
        .unwrap_or_else(|| format!("pid {}", app.pid))
}

pub(super) fn is_trusted() -> bool {
    // SAFETY: AXIsProcessTrusted is thread-safe and has no preconditions.
    unsafe { accessibility_sys::AXIsProcessTrusted() }
}

/// Whether the app's bundle id or display name contains any of the
/// (already lowercased) substrings in `filter_lc`. An empty filter
/// matches everything; that case should be short-circuited by the
/// caller, not handled here.
fn matches_bundle_filter(
    bundle_id: Option<&str>,
    app_name: Option<&str>,
    filter_lc: &[String],
) -> bool {
    let bundle_lc = bundle_id.map(str::to_lowercase);
    let name_lc = app_name.map(str::to_lowercase);
    filter_lc.iter().any(|f| {
        bundle_lc.as_deref().is_some_and(|s| s.contains(f))
            || name_lc.as_deref().is_some_and(|s| s.contains(f))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_matches_bundle_id_substring_case_insensitive() {
        let filter = vec!["rustrover".to_string()];
        assert!(matches_bundle_filter(
            Some("com.jetbrains.RustRover"),
            None,
            &filter
        ));
        assert!(matches_bundle_filter(
            Some("COM.JETBRAINS.RUSTROVER"),
            None,
            &filter
        ));
    }

    #[test]
    fn filter_matches_app_name_substring_case_insensitive() {
        let filter = vec!["chrome".to_string()];
        assert!(matches_bundle_filter(
            Some("com.google.Chrome"),
            Some("Google Chrome"),
            &filter
        ));
        assert!(matches_bundle_filter(None, Some("Google Chrome"), &filter));
    }

    #[test]
    fn filter_misses_unrelated_app() {
        let filter = vec!["webstorm".to_string()];
        assert!(!matches_bundle_filter(
            Some("com.apple.finder"),
            Some("Finder"),
            &filter
        ));
    }

    #[test]
    fn filter_or_semantics_across_multiple_terms() {
        let filter = vec!["webstorm".to_string(), "datagrip".to_string()];
        assert!(matches_bundle_filter(
            Some("com.jetbrains.WebStorm"),
            None,
            &filter
        ));
        assert!(matches_bundle_filter(
            Some("com.jetbrains.datagrip"),
            None,
            &filter
        ));
        assert!(!matches_bundle_filter(
            Some("com.apple.finder"),
            None,
            &filter
        ));
    }

    fn walk(n: usize, timed_out: bool) -> WalkResult {
        use crate::{Binding, BindingSource, Key, KeyCombo, Modifiers};
        WalkResult {
            bindings: (0..n)
                .map(|i| Binding {
                    combo: KeyCombo {
                        modifiers: Modifiers::CMD,
                        key: Key::Char('a'),
                    },
                    source: BindingSource::AppMenuItem {
                        bundle_id: "com.example".into(),
                        app_name: None,
                        menu_path: vec![],
                    },
                    label: format!("item {i}"),
                    enabled: true,
                })
                .collect(),
            timed_out,
        }
    }

    #[test]
    fn apple_menu_items_are_reported_once() {
        use crate::{Key, Modifiers};
        let lock = |label: &str| Binding {
            combo: KeyCombo {
                modifiers: Modifiers::CTRL | Modifiers::CMD,
                key: Key::Char('q'),
            },
            source: BindingSource::AppleMenuItem {
                menu_path: vec!["Apple".into(), label.into()],
            },
            label: label.into(),
            enabled: true,
        };
        let mut app_item = walk(1, false).bindings.remove(0);
        app_item.label = "Quit".into();
        let out = dedupe_apple_menu(vec![
            lock("锁定屏幕"),
            app_item,
            lock("锁定屏幕"),
            lock("退出登录"),
        ]);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn finished_retry_replaces_partial_first_walk() {
        let (best, unresponsive) = resolve_retry(walk(3, true), walk(40, false));
        assert_eq!(best.bindings.len(), 40);
        assert!(!best.timed_out);
        assert!(!unresponsive);
    }

    #[test]
    fn app_that_times_out_twice_keeps_larger_partial_and_is_flagged() {
        let (best, unresponsive) = resolve_retry(walk(7, true), walk(2, true));
        assert_eq!(best.bindings.len(), 7);
        assert!(unresponsive);
        let (best, unresponsive) = resolve_retry(walk(0, true), walk(5, true));
        assert_eq!(best.bindings.len(), 5);
        assert!(unresponsive);
    }

    #[test]
    fn retry_timeout_scales_but_keeps_system_default() {
        assert_eq!(retry_timeout(1.0), 5.0);
        assert_eq!(retry_timeout(0.5), 2.5);
        assert_eq!(retry_timeout(0.0), 0.0);
    }
}
