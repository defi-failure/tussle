use crate::{Key, Modifiers};

/// A keyboard shortcut: the set of held modifiers plus the key being pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub modifiers: Modifiers,
    pub key: Key,
}
