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

#[cfg(test)]
mod tests {
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
