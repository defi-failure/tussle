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
