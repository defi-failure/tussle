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

/// What [`capture_one`] produced.
///
/// A KeyDown event is either a normal hotkey-shaped combination (modifiers
/// plus a `kVK_*`-range key) or a macOS-internal system-action dispatch
/// code (`vk >= 0x80`); see [`SystemAction`] for why those are surfaced
/// separately rather than wrapped into `KeyCombo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Captured {
    Combo(KeyCombo),
    SystemAction(SystemAction),
}

/// A macOS extended-keycode dispatch event captured in the KeyDown channel.
///
/// macOS occasionally synthesizes KeyDown events whose virtual keycode is
/// outside the documented `kVK_*` range (`0x00..=0x7E`, see
/// `HIToolbox/Events.h`). Empirically, every such code observed so far is a
/// **system-action dispatch code** (the 🌐 key configured to switch input
/// sources, the dedicated Spotlight/Mission Control/Dictation/Do Not Disturb
/// keys on newer keyboards, etc.). These events are routed by macOS itself
/// — standard hotkey-registration APIs (`RegisterEventHotKey`, NSMenuItem
/// key equivalents, `AXMenuItemCmdChar`) do not accept them, so an app
/// cannot bind to one. In a hotkey-conflict tool the right thing to do is
/// recognize them and explain the source, not pretend they're hotkeys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SystemAction {
    /// Raw virtual keycode as reported by `kCGKeyboardEventKeycode`. Always
    /// `>= 0x80` (codes within the documented kVK_* range never become a
    /// `SystemAction`).
    pub vk: u16,
    pub kind: SystemActionKind,
}

/// Best-effort classification of an extended keycode into a known macOS
/// system action.
///
/// Apple does not document the `0x80+` virtual-keycode range. The mappings
/// here come from two sources, treated separately so we never present a
/// guess as fact:
///
///   - Verified by us: `0xA0 Mission Control` (fn+F3 default),
///     `0xB3 Change Input Source` (🌐 key).
///   - Reported by community lists (eegrok/jimratliff GitHub gists), not yet
///     verified on a machine we control: `0x81 Spotlight`,
///     `0xB0 Dictation`, `0xB2 Do Not Disturb`. We deliberately do **not**
///     add these to [`classify_extended_vk`] yet — they would turn into
///     `Unknown` and surface honestly. Promote them as we observe them
///     ourselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemActionKind {
    /// Mission Control. Default trigger is fn+F3 on Apple keyboards.
    /// Verified vk = `0xA0`.
    MissionControl,
    /// 🌐 key pressed when `System Settings → Keyboard → Press 🌐 key` is
    /// set to "Change Input Source". Verified vk = `0xB3`.
    ChangeInputSource,
    /// Any other `>= 0x80` keycode. Almost certainly a macOS system action,
    /// but not one we have classified. Surfaced verbatim so the user can
    /// report it.
    Unknown,
}

impl SystemActionKind {
    /// Human-readable action name. For [`Unknown`] it conveys the lack of
    /// classification rather than a key name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::MissionControl => "Mission Control",
            Self::ChangeInputSource => "Change Input Source",
            Self::Unknown => "unrecognized macOS extended keycode",
        }
    }

    /// Where in System Settings the user can change this action, when
    /// known. `None` for unclassified codes.
    pub fn source_hint(&self) -> Option<&'static str> {
        match self {
            Self::MissionControl => {
                Some("System Settings → Keyboard → Keyboard Shortcuts… → Mission Control")
            }
            Self::ChangeInputSource => Some("System Settings → Keyboard → Press 🌐 key"),
            Self::Unknown => None,
        }
    }
}

/// Decide whether a virtual keycode is a macOS system-action dispatch code
/// (i.e. outside the documented kVK_* range), and if so classify it.
///
/// Returns `None` for any `vk < 0x80` — those are normal keyboard codes
/// handled via [`Key::from_vk`](crate::Key::from_vk). Returns
/// `Some(SystemAction { .. })` for `vk >= 0x80`, with `kind` set to a
/// recognized variant where possible and [`SystemActionKind::Unknown`]
/// otherwise.
pub fn classify_extended_vk(vk: u16) -> Option<SystemAction> {
    if vk < 0x80 {
        return None;
    }
    let kind = match vk {
        0xA0 => SystemActionKind::MissionControl,
        0xB3 => SystemActionKind::ChangeInputSource,
        _ => SystemActionKind::Unknown,
    };
    Some(SystemAction { vk, kind })
}

/// Block until the user presses a non-modifier key, returning either the
/// resulting [`KeyCombo`] or — for macOS system-action dispatch codes —
/// a [`SystemAction`] wrapped in [`Captured`]. The captured event is
/// consumed, not propagated to whichever app or shortcut would otherwise
/// have received it.
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

    use super::{Captured, capture_error, classify_extended_vk};
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

    // Mask bits from `CoreGraphics/CGEventTypes.h` (`kCGEventFlagMask*`).
    // Identical to `NSEventModifierFlag*` because Cocoa events store the
    // same flags.
    //   kCGEventFlagMaskShift       = 1 << 17
    //   kCGEventFlagMaskControl     = 1 << 18
    //   kCGEventFlagMaskAlternate   = 1 << 19  (Option)
    //   kCGEventFlagMaskCommand     = 1 << 20
    //   kCGEventFlagMaskSecondaryFn = 1 << 23
    const FLAG_SHIFT: u64 = 1 << 17;
    const FLAG_CONTROL: u64 = 1 << 18;
    const FLAG_ALTERNATE: u64 = 1 << 19;
    const FLAG_COMMAND: u64 = 1 << 20;
    const FLAG_FUNCTION: u64 = 1 << 23;

    pub fn capture<F>(on_modifiers_changed: F) -> Result<Captured, ScanError>
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

        let captured: Arc<Mutex<Option<Captured>>> = Arc::new(Mutex::new(None));
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
                let result = match classify_extended_vk(vk) {
                    Some(action) => Captured::SystemAction(action),
                    None => Captured::Combo(KeyCombo {
                        modifiers,
                        key: Key::from_vk(vk),
                    }),
                };
                if let Ok(mut slot) = captured_for_cb.lock() {
                    *slot = Some(result);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_returns_none_for_standard_kvk_range() {
        // 0x00..=0x7F is the documented kVK_* range — handled by Key::from_vk.
        assert!(classify_extended_vk(0x00).is_none());
        assert!(classify_extended_vk(0x7E).is_none());
        assert!(classify_extended_vk(0x7F).is_none());
    }

    #[test]
    fn classify_returns_change_input_source_for_0xb3() {
        let action = classify_extended_vk(0xB3).expect("0xB3 must classify");
        assert_eq!(action.vk, 0xB3);
        assert_eq!(action.kind, SystemActionKind::ChangeInputSource);
    }

    #[test]
    fn classify_returns_mission_control_for_0xa0() {
        let action = classify_extended_vk(0xA0).expect("0xA0 must classify");
        assert_eq!(action.vk, 0xA0);
        assert_eq!(action.kind, SystemActionKind::MissionControl);
    }

    #[test]
    fn classify_returns_unknown_for_other_extended_codes() {
        let action = classify_extended_vk(0x90).expect("0x90 must classify");
        assert_eq!(action.vk, 0x90);
        assert_eq!(action.kind, SystemActionKind::Unknown);
    }

    #[test]
    fn known_kinds_have_source_hint_and_unknown_does_not() {
        assert!(SystemActionKind::ChangeInputSource.source_hint().is_some());
        assert!(SystemActionKind::Unknown.source_hint().is_none());
    }
}
