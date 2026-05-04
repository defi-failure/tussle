//! Public types describing what [`super::capture_one`] returned: either a
//! normal `KeyCombo` or a macOS system-action dispatch code.

use crate::KeyCombo;

/// What [`super::capture_one`] produced.
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
    /// Human-readable action name. For [`Self::Unknown`] it conveys the lack
    /// of classification rather than a key name.
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
