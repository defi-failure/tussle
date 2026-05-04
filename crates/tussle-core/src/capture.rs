//! One-shot interactive keystroke capture via `CGEventTap`.
//!
//! Used by `tussle who` (no args) to let the user press a hotkey and have
//! tussle echo back the [`KeyCombo`] that was pressed. The tap drops the
//! event so whichever app or system shortcut owns it does not fire —
//! tussle is asking "who has this?" not "trigger this".
//!
//! Requires Input Monitoring permission (System Settings → Privacy &
//! Security → Input Monitoring), which is separate from Accessibility.

use crate::{KeyCombo, Modifiers, ScanError};

/// Block until the user presses a non-modifier key, returning the resulting
/// [`KeyCombo`]. The captured event is consumed, not propagated to whichever
/// app or shortcut would otherwise have received it.
///
/// `on_modifiers_changed` is called every time a modifier key is pressed or
/// released while the tap is active, so the caller can render live feedback
/// (e.g. "Holding: cmd+shift...") before the final non-modifier key arrives.
pub fn capture_one_combo<F>(on_modifiers_changed: F) -> Result<KeyCombo, ScanError>
where
    F: Fn(Modifiers) + Send + 'static,
{
    #[cfg(target_os = "macos")]
    {
        platform::capture(on_modifiers_changed)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = on_modifiers_changed;
        Err(capture_error(
            "interactive capture is only supported on macOS",
        ))
    }
}

fn capture_error(msg: &str) -> ScanError {
    ScanError::Schema {
        path: std::path::PathBuf::new(),
        message: msg.into(),
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::sync::{Arc, Mutex};

    use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
        CallbackResult, EventField,
    };

    use super::capture_error;
    use crate::{Key, KeyCombo, Modifiers, ScanError};

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

    /// Mask bits from `CoreGraphics/CGEventTypes.h` (`kCGEventFlagMask*`).
    /// Identical to `NSEventModifierFlag*` because Cocoa events store the
    /// same flags.
    const FLAG_SHIFT: u64 = 1 << 17; //     kCGEventFlagMaskShift
    const FLAG_CONTROL: u64 = 1 << 18; //   kCGEventFlagMaskControl
    const FLAG_ALTERNATE: u64 = 1 << 19; // kCGEventFlagMaskAlternate (Option)
    const FLAG_COMMAND: u64 = 1 << 20; //   kCGEventFlagMaskCommand
    const FLAG_FUNCTION: u64 = 1 << 23; //  kCGEventFlagMaskSecondaryFn

    pub fn capture<F>(on_modifiers_changed: F) -> Result<KeyCombo, ScanError>
    where
        F: Fn(Modifiers) + Send + 'static,
    {
        // Trigger the Input Monitoring TCC dialog on first run (or read the
        // cached granted/denied state). Without this, CGEventTap silently
        // installs but never fires events when permission is missing.
        // SAFETY: pure C call, no preconditions.
        let granted = unsafe { IOHIDRequestAccess(KIOHID_REQUEST_TYPE_LISTEN_EVENT) != 0 };
        if !granted {
            return Err(capture_error(
                "Input Monitoring permission denied. Grant access in \
                 System Settings → Privacy & Security → Input Monitoring \
                 and re-run.",
            ));
        }

        let captured: Arc<Mutex<Option<KeyCombo>>> = Arc::new(Mutex::new(None));
        let captured_for_cb = Arc::clone(&captured);

        let runloop = CFRunLoop::get_current();
        let runloop_for_cb = runloop.clone();

        // Without Input Monitoring permission the event tap silently never
        // fires; CFRunLoopRun would block forever and Ctrl+C wouldn't even
        // reach us (the tap drops it before the terminal sees it). Install
        // a SIGINT handler that explicitly breaks the run loop.
        let runloop_for_signal = runloop.clone();
        let _ = ctrlc::set_handler(move || runloop_for_signal.stop());

        let tap = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::KeyDown, CGEventType::FlagsChanged],
            move |_proxy, etype, event| {
                let modifiers = decode_cg_flags(event.get_flags().bits());

                if matches!(etype, CGEventType::FlagsChanged) {
                    on_modifiers_changed(modifiers);
                    return CallbackResult::Drop;
                }

                // KeyDown of a non-modifier key — finalize and exit.
                let vk = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
                let combo = KeyCombo {
                    modifiers,
                    key: Key::from_vk(vk),
                };
                if let Ok(mut slot) = captured_for_cb.lock() {
                    *slot = Some(combo);
                }
                runloop_for_cb.stop();
                CallbackResult::Drop
            },
        )
        .map_err(|_| {
            capture_error("could not install event tap (Input Monitoring permission needed?)")
        })?;

        let source = tap
            .mach_port()
            .create_runloop_source(0)
            .map_err(|_| capture_error("could not create runloop source"))?;

        unsafe {
            runloop.add_source(&source, kCFRunLoopCommonModes);
        }
        tap.enable();

        CFRunLoop::run_current();

        let captured = captured
            .lock()
            .map_err(|_| capture_error("lock poisoned"))?;
        captured
            .clone()
            .ok_or_else(|| capture_error("event tap exited without capturing a key"))
    }

    fn decode_cg_flags(flags: u64) -> Modifiers {
        let mut m = Modifiers::empty();
        if flags & FLAG_SHIFT != 0 {
            m |= Modifiers::SHIFT;
        }
        if flags & FLAG_CONTROL != 0 {
            m |= Modifiers::CTRL;
        }
        if flags & FLAG_ALTERNATE != 0 {
            m |= Modifiers::OPT;
        }
        if flags & FLAG_COMMAND != 0 {
            m |= Modifiers::CMD;
        }
        if flags & FLAG_FUNCTION != 0 {
            m |= Modifiers::FN;
        }
        m
    }
}
