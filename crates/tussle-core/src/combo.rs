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

            // C0 control chars (AppKit/NSText.h). 0x7F is NSDeleteCharacter
            // — Apple's main "delete" key, semantically Backspace.
            '\u{001B}' => Key::Named(NamedKey::Escape),
            '\u{000D}' | '\u{0003}' => Key::Named(NamedKey::Return), // NSReturnCharacter / NSEnterCharacter
            '\u{0009}' => Key::Named(NamedKey::Tab),                 // NSTabCharacter
            '\u{0008}' | '\u{007F}' => Key::Named(NamedKey::Backspace), // NSBackspaceCharacter / NSDeleteCharacter

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

/// Map macOS virtual keycodes to `NamedKey`. All values verified against
/// `Carbon/HIToolbox.framework/Headers/Events.h` (kVK_* constants).
/// `kVK_Delete` is Apple's name for the main delete key — semantically
/// Backspace on every other platform.
pub(crate) fn vk_to_named(vk: u16) -> Option<NamedKey> {
    let n = match vk {
        0x31 => NamedKey::Space,     // kVK_Space
        0x24 => NamedKey::Return,    // kVK_Return
        0x30 => NamedKey::Tab,       // kVK_Tab
        0x35 => NamedKey::Escape,    // kVK_Escape
        0x33 => NamedKey::Backspace, // kVK_Delete (Apple's main delete = Backspace)
        0x75 => NamedKey::Delete,    // kVK_ForwardDelete
        0x72 => NamedKey::Help,      // kVK_Help
        0x7A => NamedKey::F1,
        0x78 => NamedKey::F2,
        0x63 => NamedKey::F3,
        0x76 => NamedKey::F4,
        0x60 => NamedKey::F5,
        0x61 => NamedKey::F6,
        0x62 => NamedKey::F7,
        0x64 => NamedKey::F8,
        0x65 => NamedKey::F9,
        0x6D => NamedKey::F10,
        0x67 => NamedKey::F11,
        0x6F => NamedKey::F12,
        0x69 => NamedKey::F13,
        0x6B => NamedKey::F14,
        0x71 => NamedKey::F15,
        0x6A => NamedKey::F16,
        0x40 => NamedKey::F17,
        0x4F => NamedKey::F18,
        0x50 => NamedKey::F19,
        0x5A => NamedKey::F20,
        0x7E => NamedKey::Up,       // kVK_UpArrow
        0x7D => NamedKey::Down,     // kVK_DownArrow
        0x7B => NamedKey::Left,     // kVK_LeftArrow
        0x7C => NamedKey::Right,    // kVK_RightArrow
        0x74 => NamedKey::PageUp,   // kVK_PageUp
        0x79 => NamedKey::PageDown, // kVK_PageDown
        0x73 => NamedKey::Home,     // kVK_Home
        0x77 => NamedKey::End,      // kVK_End
        _ => return None,
    };
    Some(n)
}

/// Map ANSI-layout virtual keycodes to their lowercase character.
/// Source: `HIToolbox/Events.h` `kVK_ANSI_*` constants.
pub(crate) fn vk_to_ansi_char(vk: u16) -> Option<char> {
    Some(match vk {
        0x00 => 'a',
        0x01 => 's',
        0x02 => 'd',
        0x03 => 'f',
        0x04 => 'h',
        0x05 => 'g',
        0x06 => 'z',
        0x07 => 'x',
        0x08 => 'c',
        0x09 => 'v',
        0x0B => 'b',
        0x0C => 'q',
        0x0D => 'w',
        0x0E => 'e',
        0x0F => 'r',
        0x10 => 'y',
        0x11 => 't',
        0x12 => '1',
        0x13 => '2',
        0x14 => '3',
        0x15 => '4',
        0x17 => '5',
        0x16 => '6',
        0x1A => '7',
        0x1C => '8',
        0x19 => '9',
        0x1D => '0',
        0x18 => '=',
        0x1B => '-',
        0x21 => '[',
        0x1E => ']',
        0x27 => '\'',
        0x29 => ';',
        0x2A => '\\',
        0x2B => ',',
        0x2F => '.',
        0x2C => '/',
        0x32 => '`',
        0x1F => 'o',
        0x20 => 'u',
        0x22 => 'i',
        0x23 => 'p',
        0x25 => 'l',
        0x26 => 'j',
        0x28 => 'k',
        0x2D => 'n',
        0x2E => 'm',
        _ => return None,
    })
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

/// Errors returned by [`KeyCombo::parse`].
#[derive(Debug, thiserror::Error)]
pub enum ComboParseError {
    #[error("empty key combo")]
    Empty,
    #[error("unknown modifier: {0:?}")]
    UnknownModifier(String),
    #[error("missing key (combo had only modifiers)")]
    MissingKey,
    #[error("unrecognized key: {0:?}")]
    UnknownKey(String),
}

impl KeyCombo {
    /// Parse a textual combo such as `cmd+opt+b`, `shift+f1`, or `escape`.
    ///
    /// Rules:
    ///   - Tokens are separated by `+` and are case-insensitive.
    ///   - All but the last token must be a modifier name:
    ///     `cmd`/`command`, `opt`/`alt`/`option`, `ctrl`/`control`, `shift`,
    ///     `fn`/`globe`. The last token is the key.
    ///   - Single-character tokens become `Key::Char` (lowercased).
    ///   - Multi-character tokens must match a `NamedKey` variant name
    ///     (`space`, `return`, `f1`, `pageup`, `escape`/`esc`, ...).
    pub fn parse(s: &str) -> Result<KeyCombo, ComboParseError> {
        let raw = s.trim();
        if raw.is_empty() {
            return Err(ComboParseError::Empty);
        }
        let tokens: Vec<&str> = raw.split('+').map(str::trim).collect();
        if tokens.iter().any(|t| t.is_empty()) {
            return Err(ComboParseError::Empty);
        }

        let (key_token, modifier_tokens) = tokens.split_last().expect("non-empty per check above");

        let mut modifiers = Modifiers::empty();
        for token in modifier_tokens {
            modifiers |= parse_modifier(token)?;
        }

        let key = parse_key(key_token)?;
        Ok(KeyCombo { modifiers, key })
    }
}

fn parse_modifier(token: &str) -> Result<Modifiers, ComboParseError> {
    Ok(match token.to_ascii_lowercase().as_str() {
        "cmd" | "command" => Modifiers::CMD,
        "opt" | "alt" | "option" => Modifiers::OPT,
        "ctrl" | "control" => Modifiers::CTRL,
        "shift" => Modifiers::SHIFT,
        "fn" | "globe" => Modifiers::FN,
        _ => return Err(ComboParseError::UnknownModifier(token.to_string())),
    })
}

fn parse_key(token: &str) -> Result<Key, ComboParseError> {
    let lower = token.to_ascii_lowercase();

    if let Some(named) = parse_named(&lower) {
        return Ok(Key::Named(named));
    }

    let mut chars = token.chars();
    let first = chars.next().ok_or(ComboParseError::MissingKey)?;
    if chars.next().is_some() {
        return Err(ComboParseError::UnknownKey(token.to_string()));
    }
    Ok(Key::Char(first.to_ascii_lowercase()))
}

fn parse_named(lower: &str) -> Option<NamedKey> {
    Some(match lower {
        "space" => NamedKey::Space,
        "return" | "enter" => NamedKey::Return,
        "tab" => NamedKey::Tab,
        "escape" | "esc" => NamedKey::Escape,
        "delete" | "del" => NamedKey::Delete,
        "backspace" => NamedKey::Backspace,
        "help" => NamedKey::Help,
        "insert" | "ins" => NamedKey::Insert,
        "f1" => NamedKey::F1,
        "f2" => NamedKey::F2,
        "f3" => NamedKey::F3,
        "f4" => NamedKey::F4,
        "f5" => NamedKey::F5,
        "f6" => NamedKey::F6,
        "f7" => NamedKey::F7,
        "f8" => NamedKey::F8,
        "f9" => NamedKey::F9,
        "f10" => NamedKey::F10,
        "f11" => NamedKey::F11,
        "f12" => NamedKey::F12,
        "f13" => NamedKey::F13,
        "f14" => NamedKey::F14,
        "f15" => NamedKey::F15,
        "f16" => NamedKey::F16,
        "f17" => NamedKey::F17,
        "f18" => NamedKey::F18,
        "f19" => NamedKey::F19,
        "f20" => NamedKey::F20,
        "up" => NamedKey::Up,
        "down" => NamedKey::Down,
        "left" => NamedKey::Left,
        "right" => NamedKey::Right,
        "pageup" => NamedKey::PageUp,
        "pagedown" => NamedKey::PageDown,
        "home" => NamedKey::Home,
        "end" => NamedKey::End,
        _ => return None,
    })
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

    mod parse {
        use super::*;

        #[test]
        fn parses_single_letter_with_cmd() {
            let c = KeyCombo::parse("cmd+b").unwrap();
            assert_eq!(c.modifiers, Modifiers::CMD);
            assert_eq!(c.key, Key::Char('b'));
        }

        #[test]
        fn parses_named_key_with_modifiers() {
            let c = KeyCombo::parse("ctrl+opt+space").unwrap();
            assert_eq!(c.modifiers, Modifiers::CTRL | Modifiers::OPT);
            assert_eq!(c.key, Key::Named(NamedKey::Space));
        }

        #[test]
        fn parses_bare_named_key() {
            let c = KeyCombo::parse("escape").unwrap();
            assert!(c.modifiers.is_empty());
            assert_eq!(c.key, Key::Named(NamedKey::Escape));
        }

        #[test]
        fn parse_is_case_insensitive() {
            assert_eq!(
                KeyCombo::parse("CMD+SHIFT+F1").unwrap(),
                KeyCombo::parse("cmd+shift+f1").unwrap(),
            );
        }

        #[test]
        fn parse_accepts_modifier_aliases() {
            let c = KeyCombo::parse("alt+option+command+a").unwrap();
            assert_eq!(c.modifiers, Modifiers::OPT | Modifiers::CMD);
            assert_eq!(c.key, Key::Char('a'));
        }

        #[test]
        fn parse_modifier_order_doesnt_matter() {
            assert_eq!(
                KeyCombo::parse("shift+cmd+3").unwrap(),
                KeyCombo::parse("cmd+shift+3").unwrap(),
            );
        }

        #[test]
        fn parse_round_trips_through_display() {
            let c = KeyCombo::parse("ctrl+shift+cmd+3").unwrap();
            assert_eq!(format!("{c}"), "ctrl+shift+cmd+3");
        }

        #[test]
        fn parse_rejects_empty() {
            assert!(matches!(KeyCombo::parse(""), Err(ComboParseError::Empty)));
            assert!(matches!(KeyCombo::parse("  "), Err(ComboParseError::Empty)));
            assert!(matches!(
                KeyCombo::parse("cmd+"),
                Err(ComboParseError::Empty)
            ));
        }

        #[test]
        fn parse_rejects_unknown_modifier() {
            assert!(matches!(
                KeyCombo::parse("super+a"),
                Err(ComboParseError::UnknownModifier(_))
            ));
        }

        #[test]
        fn parse_rejects_multichar_unnamed_key() {
            assert!(matches!(
                KeyCombo::parse("cmd+ab"),
                Err(ComboParseError::UnknownKey(_))
            ));
        }
    }
}
