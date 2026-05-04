//! macOS implementation of one-shot keystroke capture via `CGEventTap`.

mod event_tap;
mod flags;
mod keydown;
mod permission;

use crate::capture::Captured;
use crate::{Modifiers, ScanError};

/// Wait for the next non-modifier KeyDown and return what was pressed.
///
/// Returns `Err` if Input Monitoring permission is denied or the tap
/// cannot be installed.
pub(super) fn capture<F>(on_modifiers_changed: F) -> Result<Captured, ScanError>
where
    F: Fn(Modifiers) + Send + 'static,
{
    permission::check_input_monitoring()?;
    event_tap::capture_via_tap(on_modifiers_changed)
}

/// Construct a friendly `ScanError::Schema` for capture-time failures. The
/// `path` field is irrelevant here (capture isn't reading a file) — we use
/// it because `ScanError` is the crate's general error envelope.
pub(super) fn capture_error(msg: &str) -> ScanError {
    ScanError::Schema {
        path: std::path::PathBuf::new(),
        message: msg.into(),
    }
}
