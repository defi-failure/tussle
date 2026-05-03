//! Per-app menu shortcut enumeration via the macOS Accessibility API.
//!
//! Walks every running app's main menu bar and status items using
//! `AXUIElement` queries, extracting any menu item with a key equivalent.
//! Each match becomes a `Binding` with `BindingSource::AppMenuItem`.
//!
//! Requires the host process to have Accessibility permission
//! (System Settings → Privacy & Security → Accessibility). The first call
//! that exercises an `AX*` API triggers macOS's permission prompt.

use crate::{Binding, ScanError};

/// Walk every running app's menu hierarchy and collect menu-item bindings.
///
/// On non-macOS platforms this is a no-op returning an empty `Vec`.
pub fn scan() -> Result<Vec<Binding>, ScanError> {
    #[cfg(target_os = "macos")]
    {
        platform::scan()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}

/// Whether the host process currently has Accessibility permission.
///
/// On non-macOS platforms always returns `true` (no permission concept).
pub fn is_trusted() -> bool {
    #[cfg(target_os = "macos")]
    {
        platform::is_trusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    pub fn scan() -> Result<Vec<Binding>, ScanError> {
        if !is_trusted() {
            // Without permission we can't read any other app's menus.
            // Returning empty rather than erroring lets a tussle scan still
            // produce results from sources that don't need this permission.
            return Ok(Vec::new());
        }
        // TODO: enumerate running apps and walk their menu bars.
        Ok(Vec::new())
    }

    pub fn is_trusted() -> bool {
        // Safety: AXIsProcessTrusted is thread-safe and has no preconditions.
        unsafe { accessibility_sys::AXIsProcessTrusted() }
    }
}
