//! macOS implementation of the Accessibility-API menu enumerator.

mod ax;
mod menu_walker;
mod modifiers;
mod running_apps;

use crate::{Binding, ScanError};

pub(super) fn scan(messaging_timeout: f32) -> Result<Vec<Binding>, ScanError> {
    if !is_trusted() {
        return Ok(Vec::new());
    }

    let mut bindings = Vec::new();
    for app in running_apps::list_running_apps() {
        bindings.extend(menu_walker::walk_app_menus(&app, messaging_timeout));
    }
    Ok(bindings)
}

pub(super) fn is_trusted() -> bool {
    // SAFETY: AXIsProcessTrusted is thread-safe and has no preconditions.
    unsafe { accessibility_sys::AXIsProcessTrusted() }
}
