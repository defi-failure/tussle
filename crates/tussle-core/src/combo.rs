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

impl Key {
    /// Construct a `Key` from a single character, normalizing common macOS
    /// representations of non-printable keys to the corresponding
    /// `NamedKey` variant.
    ///
    /// This handles three character spaces:
    ///
    ///   - **Misc Technical glyphs** — visual symbols Apple uses in menus
    ///     (⎋ ⏎ ⇥ ⌫ ⌦ ↑↓←→ ⇞⇟ ↖↘) when an app sets `keyEquivalent` to a
    ///     literal symbol character.
    ///   - **NSText function-key constants** — Apple's PUA range
    ///     `\u{F700}–\u{F8FF}` (NSUpArrowFunctionKey, NSF1FunctionKey, etc.)
    ///     used programmatically for non-printable shortcuts.
    ///   - **C0 control characters** — `\u{1B}` ESC, `\r` Return, `\t` Tab,
    ///     `\u{08}` Backspace, `\u{7F}` Delete.
    ///
    /// Anything else is preserved as `Key::Char` (lowercased for ASCII).
    pub fn from_char(c: char) -> Key {
        match c {
            // Misc Technical glyphs (visual symbols)
            '\u{238B}' => Key::Named(NamedKey::Escape),
            '\u{23CE}' => Key::Named(NamedKey::Return),
            '\u{21E5}' => Key::Named(NamedKey::Tab),
            '\u{232B}' => Key::Named(NamedKey::Backspace),
            '\u{2326}' => Key::Named(NamedKey::Delete),
            '\u{2191}' => Key::Named(NamedKey::Up),
            '\u{2193}' => Key::Named(NamedKey::Down),
            '\u{2190}' => Key::Named(NamedKey::Left),
            '\u{2192}' => Key::Named(NamedKey::Right),
            '\u{21DE}' => Key::Named(NamedKey::PageUp),
            '\u{21DF}' => Key::Named(NamedKey::PageDown),
            '\u{2196}' => Key::Named(NamedKey::Home),
            '\u{2198}' => Key::Named(NamedKey::End),

            // C0 control chars
            '\x1B' => Key::Named(NamedKey::Escape),
            '\r' | '\u{0003}' => Key::Named(NamedKey::Return),
            '\t' => Key::Named(NamedKey::Tab),
            '\u{0008}' => Key::Named(NamedKey::Backspace),
            '\u{007F}' => Key::Named(NamedKey::Delete),

            // NSText PUA function-key constants (NSUpArrowFunctionKey, etc.)
            '\u{F700}' => Key::Named(NamedKey::Up),
            '\u{F701}' => Key::Named(NamedKey::Down),
            '\u{F702}' => Key::Named(NamedKey::Left),
            '\u{F703}' => Key::Named(NamedKey::Right),
            '\u{F704}' => Key::Named(NamedKey::F1),
            '\u{F705}' => Key::Named(NamedKey::F2),
            '\u{F706}' => Key::Named(NamedKey::F3),
            '\u{F707}' => Key::Named(NamedKey::F4),
            '\u{F708}' => Key::Named(NamedKey::F5),
            '\u{F709}' => Key::Named(NamedKey::F6),
            '\u{F70A}' => Key::Named(NamedKey::F7),
            '\u{F70B}' => Key::Named(NamedKey::F8),
            '\u{F70C}' => Key::Named(NamedKey::F9),
            '\u{F70D}' => Key::Named(NamedKey::F10),
            '\u{F70E}' => Key::Named(NamedKey::F11),
            '\u{F70F}' => Key::Named(NamedKey::F12),
            '\u{F710}' => Key::Named(NamedKey::F13),
            '\u{F711}' => Key::Named(NamedKey::F14),
            '\u{F712}' => Key::Named(NamedKey::F15),
            '\u{F713}' => Key::Named(NamedKey::F16),
            '\u{F714}' => Key::Named(NamedKey::F17),
            '\u{F715}' => Key::Named(NamedKey::F18),
            '\u{F716}' => Key::Named(NamedKey::F19),
            '\u{F717}' => Key::Named(NamedKey::F20),
            '\u{F727}' => Key::Named(NamedKey::Insert),
            '\u{F729}' => Key::Named(NamedKey::Home),
            '\u{F72B}' => Key::Named(NamedKey::End),
            '\u{F72C}' => Key::Named(NamedKey::PageUp),
            '\u{F72D}' => Key::Named(NamedKey::PageDown),
            '\u{F746}' => Key::Named(NamedKey::Help),

            other => Key::Char(other.to_ascii_lowercase()),
        }
    }
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

        #[test]
        fn from_char_normalizes_misc_technical_glyphs() {
            assert_eq!(Key::from_char('\u{238B}'), Key::Named(NamedKey::Escape));
            assert_eq!(Key::from_char('\u{23CE}'), Key::Named(NamedKey::Return));
            assert_eq!(Key::from_char('\u{21E5}'), Key::Named(NamedKey::Tab));
            assert_eq!(Key::from_char('\u{2191}'), Key::Named(NamedKey::Up));
        }

        #[test]
        fn from_char_normalizes_pua_function_keys() {
            assert_eq!(Key::from_char('\u{F700}'), Key::Named(NamedKey::Up));
            assert_eq!(Key::from_char('\u{F704}'), Key::Named(NamedKey::F1));
            assert_eq!(Key::from_char('\u{F717}'), Key::Named(NamedKey::F20));
        }

        #[test]
        fn from_char_normalizes_c0_controls() {
            assert_eq!(Key::from_char('\x1B'), Key::Named(NamedKey::Escape));
            assert_eq!(Key::from_char('\t'), Key::Named(NamedKey::Tab));
            assert_eq!(Key::from_char('\u{007F}'), Key::Named(NamedKey::Delete));
        }

        #[test]
        fn from_char_lowercases_ascii() {
            assert_eq!(Key::from_char('A'), Key::Char('a'));
            assert_eq!(Key::from_char('z'), Key::Char('z'));
        }

        #[test]
        fn from_char_preserves_other_unicode() {
            // Emoji / unusual characters used as keyEquivalent are kept
            // verbatim — the app deliberately chose this glyph.
            assert_eq!(Key::from_char('🎤'), Key::Char('🎤'));
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
