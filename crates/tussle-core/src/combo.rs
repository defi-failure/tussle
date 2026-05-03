//! Types describing a keyboard shortcut: modifiers, the key, and the
//! combo formed by their pairing.

use std::fmt::{self, Display, Formatter};

use bitflags::bitflags;

bitflags! {
    /// Set of macOS keyboard modifier flags that can be part of a hotkey.
    ///
    /// The five recognized modifiers match what every relevant macOS API
    /// exposes (Symbolic Hot Keys plist, NSUserKeyEquivalents, Carbon
    /// `RegisterEventHotKey`, Accessibility `AXMenuItemCmdModifiers`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Modifiers: u8 {
        const CMD   = 1 << 0;
        const OPT   = 1 << 1;
        const CTRL  = 1 << 2;
        const SHIFT = 1 << 3;
        const FN    = 1 << 4;
    }
}

/// Human-readable form: lowercase tokens joined with `+`, ordered
/// `ctrl+opt+shift+cmd+fn` to match the visual order Apple uses in System
/// Settings (⌃⌥⇧⌘ plus fn). Empty set renders as the empty string.
impl Display for Modifiers {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<&str> = Vec::new();
        if self.contains(Modifiers::CTRL) {
            parts.push("ctrl");
        }
        if self.contains(Modifiers::OPT) {
            parts.push("opt");
        }
        if self.contains(Modifiers::SHIFT) {
            parts.push("shift");
        }
        if self.contains(Modifiers::CMD) {
            parts.push("cmd");
        }
        if self.contains(Modifiers::FN) {
            parts.push("fn");
        }
        f.write_str(&parts.join("+"))
    }
}

/// Non-printable named key on a macOS keyboard.
///
/// Variants are added as parsers encounter them; if you need a key that's not
/// here yet, prefer adding a variant (with the corresponding macOS virtual
/// keycode handling) over falling back to `Key::Virtual`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Space,
    Return,
    Tab,
    Escape,
    Delete,
    Backspace,
    Help,
    Insert,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
}

impl Display for NamedKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            NamedKey::Space => "space",
            NamedKey::Return => "return",
            NamedKey::Tab => "tab",
            NamedKey::Escape => "escape",
            NamedKey::Delete => "delete",
            NamedKey::Backspace => "backspace",
            NamedKey::Help => "help",
            NamedKey::Insert => "insert",
            NamedKey::F1 => "f1",
            NamedKey::F2 => "f2",
            NamedKey::F3 => "f3",
            NamedKey::F4 => "f4",
            NamedKey::F5 => "f5",
            NamedKey::F6 => "f6",
            NamedKey::F7 => "f7",
            NamedKey::F8 => "f8",
            NamedKey::F9 => "f9",
            NamedKey::F10 => "f10",
            NamedKey::F11 => "f11",
            NamedKey::F12 => "f12",
            NamedKey::F13 => "f13",
            NamedKey::F14 => "f14",
            NamedKey::F15 => "f15",
            NamedKey::F16 => "f16",
            NamedKey::F17 => "f17",
            NamedKey::F18 => "f18",
            NamedKey::F19 => "f19",
            NamedKey::F20 => "f20",
            NamedKey::Up => "up",
            NamedKey::Down => "down",
            NamedKey::Left => "left",
            NamedKey::Right => "right",
            NamedKey::PageUp => "pageup",
            NamedKey::PageDown => "pagedown",
            NamedKey::Home => "home",
            NamedKey::End => "end",
        };
        f.write_str(s)
    }
}

/// The non-modifier portion of a hotkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A single printable character (letter, digit, or punctuation).
    Char(char),
    /// A recognized non-printable key.
    Named(NamedKey),
    /// A macOS virtual keycode we have not yet classified into a named or
    /// printable variant. Surfaced to the user verbatim so unmapped keys are
    /// visible rather than silently dropped.
    Virtual(u16),
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Key::Char(c) => write!(f, "{c}"),
            Key::Named(n) => Display::fmt(n, f),
            Key::Virtual(v) => write!(f, "vk{v}"),
        }
    }
}

/// A keyboard shortcut: the set of held modifiers plus the key being pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub modifiers: Modifiers,
    pub key: Key,
}

/// Renders as `modifiers+key`, e.g. `cmd+space` or `ctrl+shift+cmd+3`. When
/// the modifier set is empty, only the key is rendered (`escape`, `f1`).
impl Display for KeyCombo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.modifiers.is_empty() {
            Display::fmt(&self.key, f)
        } else {
            write!(f, "{}+{}", self.modifiers, self.key)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod modifiers {
        use super::*;

        #[test]
        fn display_empty_is_empty_string() {
            assert_eq!(format!("{}", Modifiers::empty()), "");
        }

        #[test]
        fn display_single_flag() {
            assert_eq!(format!("{}", Modifiers::CMD), "cmd");
        }

        #[test]
        fn display_orders_ctrl_opt_shift_cmd_fn() {
            let m = Modifiers::CMD | Modifiers::SHIFT | Modifiers::CTRL;
            assert_eq!(format!("{m}"), "ctrl+shift+cmd");
        }

        #[test]
        fn display_all_flags() {
            let m = Modifiers::all();
            assert_eq!(format!("{m}"), "ctrl+opt+shift+cmd+fn");
        }
    }

    mod key {
        use super::*;

        #[test]
        fn display_char() {
            assert_eq!(format!("{}", Key::Char('a')), "a");
        }

        #[test]
        fn display_named() {
            assert_eq!(format!("{}", Key::Named(NamedKey::Space)), "space");
            assert_eq!(format!("{}", Key::Named(NamedKey::F1)), "f1");
            assert_eq!(format!("{}", Key::Named(NamedKey::PageUp)), "pageup");
        }

        #[test]
        fn display_virtual_uses_vk_prefix() {
            assert_eq!(format!("{}", Key::Virtual(42)), "vk42");
        }
    }

    mod key_combo {
        use super::*;

        #[test]
        fn display_with_single_modifier() {
            let c = KeyCombo {
                modifiers: Modifiers::CMD,
                key: Key::Named(NamedKey::Space),
            };
            assert_eq!(format!("{c}"), "cmd+space");
        }

        #[test]
        fn display_with_multiple_modifiers() {
            let c = KeyCombo {
                modifiers: Modifiers::CMD | Modifiers::SHIFT | Modifiers::CTRL,
                key: Key::Char('3'),
            };
            assert_eq!(format!("{c}"), "ctrl+shift+cmd+3");
        }

        #[test]
        fn display_without_modifiers_is_just_key() {
            let c = KeyCombo {
                modifiers: Modifiers::empty(),
                key: Key::Named(NamedKey::Escape),
            };
            assert_eq!(format!("{c}"), "escape");
        }
    }
}
