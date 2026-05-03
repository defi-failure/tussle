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
pub fn scan() -> Result<Vec<Binding>, ScanError> {
    todo!("accessibility scan not yet implemented")
}
