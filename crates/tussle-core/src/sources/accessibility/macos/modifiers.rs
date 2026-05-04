//! `AXMenuItemCmdModifiers` decoding.

use crate::Modifiers;

// `AXMenuItemModifiers` from `HIServices/AXAttributeConstants.h`:
//   kAXMenuItemModifierShift     = 1 << 0
//   kAXMenuItemModifierOption    = 1 << 1
//   kAXMenuItemModifierControl   = 1 << 2
//   kAXMenuItemModifierNoCommand = 1 << 3
const AX_MOD_SHIFT: i64 = 1 << 0;
const AX_MOD_OPTION: i64 = 1 << 1;
const AX_MOD_CONTROL: i64 = 1 << 2;
const AX_MOD_NO_COMMAND: i64 = 1 << 3;

/// Decode an `AXMenuItemCmdModifiers` integer.
///
/// Cmd is **implicit** for menu shortcuts and we add it unless the
/// no-command bit is set (which apps use for non-Cmd shortcuts like
/// PixPin's ⌃1 / ⌃2).
pub(super) fn decode_ax_modifiers(mask: i64) -> Modifiers {
    let mut m = Modifiers::empty();
    if mask & AX_MOD_SHIFT != 0 {
        m |= Modifiers::SHIFT;
    }
    if mask & AX_MOD_OPTION != 0 {
        m |= Modifiers::OPT;
    }
    if mask & AX_MOD_CONTROL != 0 {
        m |= Modifiers::CTRL;
    }
    if mask & AX_MOD_NO_COMMAND == 0 {
        m |= Modifiers::CMD;
    }
    m
}
