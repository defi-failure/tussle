//! One-shot interactive keystroke capture via `CGEventTap`.
//!
//! Used by `tussle who` (no args) to let the user press a hotkey and have
//! tussle echo back the [`KeyCombo`] that was pressed. The tap drops the
//! event so whichever app or system shortcut owns it does not fire —
//! tussle is asking "who has this?" not "trigger this".
//!
//! Requires Input Monitoring permission (System Settings → Privacy &
//! Security → Input Monitoring), which is separate from Accessibility.

use crate::{KeyCombo, ScanError};

/// Block until the user presses a non-modifier key, returning the resulting
/// [`KeyCombo`]. The captured event is consumed, not propagated to whichever
/// app or shortcut would otherwise have received it.
pub fn capture_one_combo() -> Result<KeyCombo, ScanError> {
    #[cfg(target_os = "macos")]
    {
        platform::capture()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(capture_error("interactive capture is only supported on macOS"))
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
    use crate::sources::symbolichotkeys::vk_to_named;
    use crate::{Key, KeyCombo, Modifiers, ScanError};

    /// Mask bits from `CoreGraphics/CGEventTypes.h` (`kCGEventFlagMask*`).
    /// Identical to `NSEventModifierFlag*` because Cocoa events store the
    /// same flags.
    const FLAG_SHIFT: u64 = 1 << 17; //     kCGEventFlagMaskShift
    const FLAG_CONTROL: u64 = 1 << 18; //   kCGEventFlagMaskControl
    const FLAG_ALTERNATE: u64 = 1 << 19; // kCGEventFlagMaskAlternate (Option)
    const FLAG_COMMAND: u64 = 1 << 20; //   kCGEventFlagMaskCommand
    const FLAG_FUNCTION: u64 = 1 << 23; //  kCGEventFlagMaskSecondaryFn

    pub fn capture() -> Result<KeyCombo, ScanError> {
        let captured: Arc<Mutex<Option<KeyCombo>>> = Arc::new(Mutex::new(None));
        let captured_for_cb = Arc::clone(&captured);

        let runloop = CFRunLoop::get_current();
        let runloop_for_cb = runloop.clone();

        let tap = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::KeyDown],
            move |_proxy, _etype, event| {
                let vk =
                    event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
                let flags = event.get_flags().bits();
                let combo = KeyCombo {
                    modifiers: decode_cg_flags(flags),
                    key: vk_to_key(vk),
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

        let captured = captured.lock().map_err(|_| capture_error("lock poisoned"))?;
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

    fn vk_to_key(vk: u16) -> Key {
        if let Some(named) = vk_to_named(vk) {
            return Key::Named(named);
        }
        Key::Virtual(vk)
    }
}
