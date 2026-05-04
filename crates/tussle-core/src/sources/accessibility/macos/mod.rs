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
    let apps = running_apps::list_running_apps();
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
