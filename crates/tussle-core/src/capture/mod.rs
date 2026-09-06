//! One-shot interactive keystroke capture via `CGEventTap`.
//!
//! Used by `tussle who` (no args) to let the user press a hotkey and have
//! tussle echo back the [`Captured`] value. The tap drops the event so
//! whichever app or system shortcut owns it does not fire — tussle is
//! asking "who has this?" not "trigger this".
//!
//! Requires Input Monitoring permission (System Settings → Privacy &
//! Security → Input Monitoring), which is separate from Accessibility.

mod types;

pub use types::{
    Captured, Probe, Reaction, ReactionKind, SystemAction, SystemActionKind, classify_extended_vk,
};

#[cfg(target_os = "macos")]
mod macos;

use crate::{Modifiers, ScanError};

/// Block until the user presses a non-modifier key, returning either the
/// resulting [`KeyCombo`](crate::KeyCombo) or — for macOS system-action
/// dispatch codes — a [`SystemAction`] wrapped in [`Captured`]. The captured
/// event is consumed, not propagated to whichever app or shortcut would
/// otherwise have received it.
///
/// `on_modifiers_changed` is called every time a modifier key is pressed or
/// released while the tap is active, so the caller can render live feedback
/// (e.g. "Holding: cmd+shift...") before the final non-modifier key arrives.
pub fn capture_one<F>(on_modifiers_changed: F) -> Result<Captured, ScanError>
where
    F: Fn(Modifiers) + Send + 'static,
{
    #[cfg(target_os = "macos")]
    {
        macos::capture(on_modifiers_changed)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = on_modifiers_changed;
        Err(ScanError::Schema {
            path: std::path::PathBuf::new(),
            message: "interactive capture is only supported on macOS".into(),
        })
    }
}

/// State of a macOS privacy permission, read without prompting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionStatus {
    Granted,
    Denied,
    /// The user has not been asked yet; macOS will prompt on first use.
    Undetermined,
}

/// Whether this process may install a keyboard event tap (Input
/// Monitoring). Never prompts; `capture_one` does.
pub fn input_monitoring_status() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        macos::input_monitoring_status()
    }
    #[cfg(not(target_os = "macos"))]
    {
        PermissionStatus::Granted
    }
}

/// Wait for the next keystroke, let it through, and watch for `settle` who
/// reacts: apps coming to the front, new windows, an input source change.
///
/// Unlike [`capture_one`] the key is not swallowed, so whatever it is
/// bound to actually happens.
pub fn capture_and_probe<F>(
    on_modifiers_changed: F,
    settle: std::time::Duration,
) -> Result<Probe, ScanError>
where
    F: Fn(Modifiers) + Send + 'static,
{
    #[cfg(target_os = "macos")]
    {
        macos::probe(on_modifiers_changed, settle)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (on_modifiers_changed, settle);
        Err(ScanError::Schema {
            path: std::path::PathBuf::new(),
            message: "interactive capture is only supported on macOS".into(),
        })
    }
}
