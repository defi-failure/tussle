use std::fmt::{self, Display, Formatter};

use crate::{Key, Modifiers};

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
    use crate::NamedKey;

    #[test]
    fn display_combo_with_single_modifier() {
        let c = KeyCombo {
            modifiers: Modifiers::CMD,
            key: Key::Named(NamedKey::Space),
        };
        assert_eq!(format!("{c}"), "cmd+space");
    }

    #[test]
    fn display_combo_with_multiple_modifiers() {
        let c = KeyCombo {
            modifiers: Modifiers::CMD | Modifiers::SHIFT | Modifiers::CTRL,
            key: Key::Char('3'),
        };
        assert_eq!(format!("{c}"), "ctrl+shift+cmd+3");
    }

    #[test]
    fn display_combo_without_modifiers_is_just_key() {
        let c = KeyCombo {
            modifiers: Modifiers::empty(),
            key: Key::Named(NamedKey::Escape),
        };
        assert_eq!(format!("{c}"), "escape");
    }
}
