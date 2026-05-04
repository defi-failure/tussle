//! Per-app menu shortcut enumeration via the macOS Accessibility API.
//!
//! Walks every running app's main menu bar and status items using
//! `AXUIElement` queries, extracting any menu item with a key equivalent.
//! Each match becomes a `Binding` with `BindingSource::AppMenuItem`.
//!
//! Requires the host process to have Accessibility permission
//! (System Settings → Privacy & Security → Accessibility). The first call
//! that exercises an `AX*` API triggers macOS's permission prompt.

#[cfg(target_os = "macos")]
mod macos;

use crate::{Binding, ScanError};

use super::Source;

/// Source backed by the macOS Accessibility API.
#[derive(Debug, Clone, Copy)]
pub struct Accessibility {
    /// Per-app `AXUIElementSetMessagingTimeout`, in seconds. Values `<= 0`
    /// leave the system default in place. Tight values (e.g. 1.0) prevent a
    /// single non-responsive app from stalling the whole scan.
    pub messaging_timeout: f32,
    /// Defensive cap on the number of apps walked in parallel. `0` means
    /// no cap (one OS thread per app, all at once). Default 128 — at the
    /// typical 50–100 running apps this is effectively unbounded; only
    /// pathological sessions (hundreds of processes) ever hit the chunked
    /// path.
    pub max_concurrency: usize,
}

impl Default for Accessibility {
    fn default() -> Self {
        Self {
            messaging_timeout: 1.0,
            max_concurrency: 128,
        }
    }
}

impl Accessibility {
    pub fn new(messaging_timeout: f32, max_concurrency: usize) -> Self {
        Self {
            messaging_timeout,
            max_concurrency,
        }
    }
}

impl Source for Accessibility {
    fn name(&self) -> &'static str {
        "accessibility"
    }

    fn scan(&self) -> Result<Vec<Binding>, ScanError> {
        #[cfg(target_os = "macos")]
        {
            macos::scan(self.messaging_timeout, self.max_concurrency)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(Vec::new())
        }
    }
}

/// Whether the host process currently has Accessibility permission.
///
/// On non-macOS platforms always returns `true` (no permission concept).
pub fn is_trusted() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::is_trusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}
