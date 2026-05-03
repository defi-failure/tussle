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
