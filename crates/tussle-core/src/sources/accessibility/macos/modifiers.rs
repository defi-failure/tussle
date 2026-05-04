//! `AXMenuItemCmdModifiers` decoding.

use crate::Modifiers;

// `AXMenuItemModifiers` from `HIServices/AXAttributeConstants.h`:
//   kAXMenuItemModifierShift     = 1 << 0
//   kAXMenuItemModifierOption    = 1 << 1
//   kAXMenuItemModifierControl   = 1 << 2
//   kAXMenuItemModifierNoCommand = 1 << 3
//
// `1 << 4` is **undocumented** but empirically a fn / Globe modifier:
// macOS 14+ uses `fn+F` as the default Enter Full Screen shortcut, and
// AX reports those items with mask = `0x10` or `0x18` (fn + NO_COMMAND).
// Without recognizing this bit, fn-modified bindings collapse into
// "single key" rows like a bare `f`.
const AX_MOD_SHIFT: i64 = 1 << 0;
const AX_MOD_OPTION: i64 = 1 << 1;
const AX_MOD_CONTROL: i64 = 1 << 2;
const AX_MOD_NO_COMMAND: i64 = 1 << 3;
const AX_MOD_FN: i64 = 1 << 4;

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
    if mask & AX_MOD_FN != 0 {
        m |= Modifiers::FN;
    }
    if mask & AX_MOD_NO_COMMAND == 0 {
        m |= Modifiers::CMD;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_mask_is_implicit_cmd() {
        // mask=0: cmd is implicit, nothing else set.
        assert_eq!(decode_ax_modifiers(0), Modifiers::CMD);
    }

    #[test]
    fn no_command_alone_is_empty() {
        // PixPin-style raw key (e.g. ⌃1) — modifiers fully explicit, no cmd.
        assert_eq!(decode_ax_modifiers(AX_MOD_NO_COMMAND), Modifiers::empty());
    }

    #[test]
    fn fn_bit_decodes_to_fn_modifier() {
        // macOS 14+ "Enter Full Screen" = fn+f, mask = 0x18.
        assert_eq!(
            decode_ax_modifiers(AX_MOD_FN | AX_MOD_NO_COMMAND),
            Modifiers::FN
        );
    }

    #[test]
    fn ctrl_with_implicit_cmd() {
        assert_eq!(
            decode_ax_modifiers(AX_MOD_CONTROL),
            Modifiers::CTRL | Modifiers::CMD
        );
    }
}
