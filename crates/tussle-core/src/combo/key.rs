//! The non-modifier portion of a hotkey: printable chars, named keys, or
//! a verbatim macOS virtual keycode for unmapped values.

use std::fmt::{self, Display, Formatter};

use super::vk::{vk_to_ansi_char, vk_to_named};

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
    /// Sources for the codepoints below:
    ///
    ///   - **Misc Technical glyphs** are the visual symbols Apple uses in
    ///     menus (⎋⏎⇥⌫⌦↑↓←→⇞⇟↖↘). Apple does not formally document each
    ///     codepoint, but `U+238B` (Escape) and `U+23CE` (Return) are well-
    ///     established conventions widely cited across Apple developer
    ///     references and community lists.
    ///   - **NSText PUA function-key constants** come from `AppKit/NSEvent.h`
    ///     (e.g. `NSUpArrowFunctionKey = 0xF700`, `NSF1FunctionKey = 0xF704`,
    ///     `NSDeleteFunctionKey = 0xF728`, `NSHelpFunctionKey = 0xF746`).
    ///   - **C0 control characters** come from `AppKit/NSText.h`:
    ///     `NSBackspaceCharacter = 0x08`, `NSTabCharacter = 0x09`,
    ///     `NSReturnCharacter = 0x0D`, `NSEnterCharacter = 0x03`,
    ///     `NSDeleteCharacter = 0x7F` (which on Apple keyboards is the main
    ///     "delete" key — semantically what most platforms call backspace).
    ///
    /// Anything else is preserved as `Key::Char` (lowercased for ASCII).
    pub fn from_char(c: char) -> Key {
        match c {
            // Misc Technical glyphs (visual symbols Apple uses in menus).
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

            // C0 control chars (AppKit/NSText.h):
            //   ESC                   = 0x1B
            //   NSEnterCharacter      = 0x03   (numeric-keypad enter)
            //   NSReturnCharacter     = 0x0D
            //   NSTabCharacter        = 0x09
            //   NSBackspaceCharacter  = 0x08
            //   NSDeleteCharacter     = 0x7F   (Apple's main delete = Backspace)
            '\u{001B}' => Key::Named(NamedKey::Escape),
            // 0x0A (LF) observed in real menus — Apple's NSText.h doesn't
            // formally pin LF, but semantically it groups with Return/Enter.
            '\u{000D}' | '\u{000A}' | '\u{0003}' => Key::Named(NamedKey::Return),
            '\u{0009}' => Key::Named(NamedKey::Tab),
            '\u{0008}' | '\u{007F}' => Key::Named(NamedKey::Backspace),

            // Plain space — Apple's `kAXMenuItemCmdChar` for space-bar
            // shortcuts (e.g. JetBrains' Basic Completion = ctrl+space) is
            // literally `" "`. Without this case we'd render `ctrl+ ` and
            // it would look like the key was missing.
            ' ' => Key::Named(NamedKey::Space),

            // NSText PUA function-key constants from AppKit/NSEvent.h:
            //   NSUpArrowFunctionKey    = 0xF700
            //   NSDownArrowFunctionKey  = 0xF701
            //   NSLeftArrowFunctionKey  = 0xF702
            //   NSRightArrowFunctionKey = 0xF703
            //   NSF1FunctionKey..NSF20FunctionKey = 0xF704..0xF717
            //   NSInsertFunctionKey     = 0xF727
            //   NSDeleteFunctionKey     = 0xF728  (forward-delete)
            //   NSHomeFunctionKey       = 0xF729
            //   NSEndFunctionKey        = 0xF72B
            //   NSPageUpFunctionKey     = 0xF72C
            //   NSPageDownFunctionKey   = 0xF72D
            //   NSHelpFunctionKey       = 0xF746
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
            '\u{F728}' => Key::Named(NamedKey::Delete),
            '\u{F729}' => Key::Named(NamedKey::Home),
            '\u{F72B}' => Key::Named(NamedKey::End),
            '\u{F72C}' => Key::Named(NamedKey::PageUp),
            '\u{F72D}' => Key::Named(NamedKey::PageDown),
            '\u{F746}' => Key::Named(NamedKey::Help),

            other => Key::Char(other.to_ascii_lowercase()),
        }
    }

    /// Construct a `Key` from a macOS Carbon virtual keycode.
    ///
    /// The mapping covers three groups, all from
    /// `HIToolbox.framework/Headers/Events.h`:
    ///
    ///   - **ANSI letter / digit / punctuation keys** (`kVK_ANSI_*`) →
    ///     `Key::Char` of the US-ANSI character. This is the convention
    ///     macOS shortcut notation follows regardless of the user's actual
    ///     keyboard layout.
    ///   - **Named non-printable keys** (`kVK_Space`, `kVK_F1`, etc.) →
    ///     `Key::Named`.
    ///   - Anything else → `Key::Virtual(vk)` so the caller still sees the
    ///     raw code rather than a silent drop.
    pub fn from_vk(vk: u16) -> Key {
        if let Some(c) = vk_to_ansi_char(vk) {
            return Key::Char(c);
        }
        if let Some(named) = vk_to_named(vk) {
            return Key::Named(named);
        }
        Key::Virtual(vk)
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

#[cfg(test)]
mod tests {
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
        // 0x7F is NSDeleteCharacter — Apple's main "delete" key, which
        // every other platform calls Backspace.
        assert_eq!(Key::from_char('\u{007F}'), Key::Named(NamedKey::Backspace));
        assert_eq!(Key::from_char('\u{0008}'), Key::Named(NamedKey::Backspace));
    }

    #[test]
    fn from_char_distinguishes_backspace_from_forward_delete() {
        // NSBackspaceCharacter / NSDeleteCharacter both → Backspace.
        assert_eq!(Key::from_char('\u{0008}'), Key::Named(NamedKey::Backspace));
        assert_eq!(Key::from_char('\u{007F}'), Key::Named(NamedKey::Backspace));
        // NSDeleteFunctionKey is the dedicated forward-delete key.
        assert_eq!(Key::from_char('\u{F728}'), Key::Named(NamedKey::Delete));
    }

    #[test]
    fn from_char_lowercases_ascii() {
        assert_eq!(Key::from_char('A'), Key::Char('a'));
        assert_eq!(Key::from_char('z'), Key::Char('z'));
    }

    #[test]
    fn from_char_normalizes_space() {
        assert_eq!(Key::from_char(' '), Key::Named(NamedKey::Space));
    }

    #[test]
    fn from_char_normalizes_lf_to_return() {
        // 0x0A turns up in real `kAXMenuItemCmdChar` reads alongside 0x0D;
        // both are surfaced as Return.
        assert_eq!(Key::from_char('\n'), Key::Named(NamedKey::Return));
        assert_eq!(Key::from_char('\u{000A}'), Key::Named(NamedKey::Return));
    }

    #[test]
    fn from_char_preserves_other_unicode() {
        // Emoji / unusual characters used as keyEquivalent are kept
        // verbatim — the app deliberately chose this glyph.
        assert_eq!(Key::from_char('🎤'), Key::Char('🎤'));
    }

    #[test]
    fn from_vk_ansi_letter_keys() {
        // kVK_ANSI_A = 0x00 → 'a'
        assert_eq!(Key::from_vk(0x00), Key::Char('a'));
        // kVK_ANSI_C = 0x08 → 'c'
        assert_eq!(Key::from_vk(0x08), Key::Char('c'));
        // kVK_ANSI_M = 0x2E → 'm'
        assert_eq!(Key::from_vk(0x2E), Key::Char('m'));
    }

    #[test]
    fn from_vk_ansi_digit_keys() {
        // kVK_ANSI_1 = 0x12 → '1'
        assert_eq!(Key::from_vk(0x12), Key::Char('1'));
        // kVK_ANSI_0 = 0x1D → '0'
        assert_eq!(Key::from_vk(0x1D), Key::Char('0'));
    }

    #[test]
    fn from_vk_named_keys() {
        assert_eq!(Key::from_vk(0x31), Key::Named(NamedKey::Space));
        assert_eq!(Key::from_vk(0x35), Key::Named(NamedKey::Escape));
        assert_eq!(Key::from_vk(0x7A), Key::Named(NamedKey::F1));
        assert_eq!(Key::from_vk(0x7E), Key::Named(NamedKey::Up));
    }

    #[test]
    fn from_vk_unknown_falls_back_to_virtual() {
        assert_eq!(Key::from_vk(0xFFFE), Key::Virtual(0xFFFE));
    }
}
