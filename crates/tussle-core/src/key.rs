use std::fmt::{self, Display, Formatter};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_char_key() {
        assert_eq!(format!("{}", Key::Char('a')), "a");
    }

    #[test]
    fn display_named_key() {
        assert_eq!(format!("{}", Key::Named(NamedKey::Space)), "space");
        assert_eq!(format!("{}", Key::Named(NamedKey::F1)), "f1");
        assert_eq!(format!("{}", Key::Named(NamedKey::PageUp)), "pageup");
    }

    #[test]
    fn display_virtual_key_uses_vk_prefix() {
        assert_eq!(format!("{}", Key::Virtual(42)), "vk42");
    }
}
