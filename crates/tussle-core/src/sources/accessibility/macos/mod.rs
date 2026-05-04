//! macOS implementation of the Accessibility-API menu enumerator.

mod ax;
mod menu_walker;
mod modifiers;
mod running_apps;

use std::thread;

use crate::{Binding, ScanError};

pub(super) fn scan(
    messaging_timeout: f32,
    max_concurrency: usize,
    bundle_filter: &[String],
) -> Result<Vec<Binding>, ScanError> {
    if !is_trusted() {
        tracing::warn!("Accessibility permission missing — skipping menu enumeration");
        return Ok(Vec::new());
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

    let mut bindings = Vec::new();
    for batch in apps.chunks(chunk_size) {
        let batch_bindings: Vec<Binding> = thread::scope(|s| {
            let handles: Vec<_> = batch
                .iter()
                .map(|app| s.spawn(move || menu_walker::walk_app_menus(app, messaging_timeout)))
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap_or_default())
                .collect()
        });
        bindings.extend(batch_bindings);
    }
    Ok(bindings)
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
        assert!(matches_bundle_filter(
            None,
            Some("Google Chrome"),
            &filter
        ));
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
}
