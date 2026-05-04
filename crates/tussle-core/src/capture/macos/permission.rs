//! Input Monitoring permission gate (TCC).

use crate::ScanError;

use super::capture_error;

// FFI binding to IOKit's `IOHIDRequestAccess`. From
// `IOKit/hidsystem/IOHIDLib.h` (verified against macOS 15.4/26.1 SDK):
//
//   typedef enum {
//       kIOHIDRequestTypePostEvent   = 0,
//       kIOHIDRequestTypeListenEvent = 1,
//   } IOHIDRequestType;
//   Boolean IOHIDRequestAccess(IOHIDRequestType requestType);
//
// Calling this triggers macOS's Input Monitoring permission dialog the
// first time access status is undecided, and returns the cached result
// after the user has decided. Without it, CGEventTap silently never
// fires when permission is missing.
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOHIDRequestAccess(requestType: u32) -> u8;
}
const KIOHID_REQUEST_TYPE_LISTEN_EVENT: u32 = 1;

/// Block until macOS resolves the Input Monitoring TCC prompt, then return
/// `Ok(())` if granted or a friendly `ScanError` if denied.
pub(super) fn check_input_monitoring() -> Result<(), ScanError> {
    // SAFETY: pure C call, no preconditions.
    let granted = unsafe { IOHIDRequestAccess(KIOHID_REQUEST_TYPE_LISTEN_EVENT) != 0 };
    if granted {
        Ok(())
    } else {
        Err(capture_error(
            "Input Monitoring permission denied. Grant access in \
             System Settings → Privacy & Security → Input Monitoring \
             and re-run.",
        ))
    }
}
