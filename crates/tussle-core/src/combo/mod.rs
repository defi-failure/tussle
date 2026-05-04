//! Types describing a keyboard shortcut: modifiers, the key, and the
//! combo formed by their pairing.

mod key;
mod modifiers;
mod parse;
mod vk;

pub use key::{Key, NamedKey};
pub use modifiers::Modifiers;

pub(crate) use vk::vk_to_named;

use std::fmt::{self, Display, Formatter};

/// A keyboard shortcut: the set of held modifiers plus the key being pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub modifiers: Modifiers,
    pub key: Key,
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
