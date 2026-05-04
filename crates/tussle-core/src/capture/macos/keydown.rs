//! Pure conversion from a captured KeyDown event's `(vk, modifiers)` pair
//! into a [`Captured`].

use crate::capture::{Captured, classify_extended_vk};
use crate::{Key, KeyCombo, Modifiers};

/// Build a `Captured` from a raw KeyDown event. macOS extended keycodes
/// (`vk >= 0x80`) become `Captured::SystemAction`; everything else becomes
/// `Captured::Combo`.
pub(super) fn build_captured(vk: u16, modifiers: Modifiers) -> Captured {
    match classify_extended_vk(vk) {
        Some(action) => Captured::SystemAction(action),
        None => Captured::Combo(KeyCombo {
            modifiers,
            key: Key::from_vk(vk),
        }),
    }
}
