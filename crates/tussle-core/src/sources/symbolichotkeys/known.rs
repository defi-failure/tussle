//! What we know about macOS's own shortcuts beyond what the live table
//! says: which id a combo belongs to, its label, whether macOS ships it
//! switched on, and whether macOS handles it before apps or merely offers
//! it as a standard menu item.
//!
//! **Maintenance** (once per macOS major release): compare against
//! System Settings → Keyboard → Keyboard Shortcuts and the labels in
//! `KeyboardSettings.appex/Contents/Resources/DefaultShortcutsTable.loctable`.
//! Only list what has been verified; an unknown entry is reported as a
//! generic macOS shortcut, which is honest, whereas a wrong label or a
//! wrong dispatch is not.

use crate::{Key, KeyCombo, Modifiers, NamedKey, SystemDispatch};

/// A symbolic hotkey that has an id in `com.apple.symbolichotkeys`.
pub(super) struct KnownHotkey {
    pub id: u32,
    /// Built-in combo, when there is one; some ids ship with no key.
    pub combo: Option<KeyCombo>,
    /// Whether macOS ships the shortcut switched on. A user who never
    /// touched it has no plist entry, so this is the state that applies
    /// when the live table is unavailable.
    pub enabled: bool,
    pub label: &'static str,
    pub dispatch: SystemDispatch,
}

/// A shortcut macOS registers without exposing an id.
pub(super) struct BuiltinShortcut {
    pub combo: KeyCombo,
    pub label: &'static str,
    pub dispatch: SystemDispatch,
}

fn combo(modifiers: Modifiers, key: Key) -> KeyCombo {
    KeyCombo { modifiers, key }
}

fn ch(modifiers: Modifiers, c: char) -> KeyCombo {
    combo(modifiers, Key::Char(c))
}

fn named(modifiers: Modifiers, k: NamedKey) -> KeyCombo {
    combo(modifiers, Key::Named(k))
}

/// Verified against macOS Tahoe (26.x).
pub(super) fn known_hotkeys() -> Vec<KnownHotkey> {
    use NamedKey::*;
    use SystemDispatch::{BeforeApps, StandardMenuItem};
    let cmd = Modifiers::CMD;
    let shift = Modifiers::SHIFT;
    let ctrl = Modifiers::CTRL;
    let opt = Modifiers::OPT;
    let k = |id, combo, enabled, label, dispatch| KnownHotkey {
        id,
        combo,
        enabled,
        label,
        dispatch,
    };

    vec![
        // Keyboard navigation. The window server moves focus itself.
        k(
            7,
            Some(named(ctrl, F2)),
            true,
            "Move focus to the menu bar",
            BeforeApps,
        ),
        k(
            8,
            Some(named(ctrl, F3)),
            true,
            "Move focus to the Dock",
            BeforeApps,
        ),
        k(
            9,
            Some(named(ctrl, F4)),
            true,
            "Move focus to active or next window",
            BeforeApps,
        ),
        k(
            10,
            Some(named(ctrl, F5)),
            true,
            "Move focus to the window toolbar",
            BeforeApps,
        ),
        k(
            11,
            Some(named(ctrl, F6)),
            true,
            "Move focus to the floating window",
            BeforeApps,
        ),
        k(
            12,
            Some(named(ctrl, F1)),
            true,
            "Turn keyboard access on or off",
            BeforeApps,
        ),
        k(
            13,
            Some(named(ctrl, F7)),
            true,
            "Change the way Tab moves focus",
            BeforeApps,
        ),
        k(
            57,
            Some(named(ctrl, F8)),
            true,
            "Move focus to status menus",
            BeforeApps,
        ),
        // "Move focus to next window" is the Window menu's own Cycle
        // Through Windows item, so an app that rebinds ⌘` keeps it.
        k(
            27,
            Some(ch(cmd, '`')),
            true,
            "Move focus to next window",
            StandardMenuItem,
        ),
        k(
            51,
            None,
            true,
            "Move focus to the window drawer",
            StandardMenuItem,
        ),
        // Accessibility zoom and display. All ship switched off.
        k(
            15,
            Some(ch(opt | cmd, '8')),
            false,
            "Turn zoom on or off",
            BeforeApps,
        ),
        k(17, Some(ch(opt | cmd, '=')), false, "Zoom in", BeforeApps),
        k(19, Some(ch(opt | cmd, '-')), false, "Zoom out", BeforeApps),
        k(
            21,
            Some(ch(ctrl | opt | cmd, '8')),
            false,
            "Invert colors",
            BeforeApps,
        ),
        k(
            23,
            Some(ch(opt | cmd, '\\')),
            false,
            "Turn image smoothing on or off",
            BeforeApps,
        ),
        k(
            25,
            Some(ch(ctrl | opt | cmd, '.')),
            false,
            "Increase contrast",
            BeforeApps,
        ),
        k(
            26,
            Some(ch(ctrl | opt | cmd, ',')),
            false,
            "Decrease contrast",
            BeforeApps,
        ),
        k(
            59,
            Some(named(cmd, F5)),
            true,
            "Turn VoiceOver on or off",
            BeforeApps,
        ),
        // Screenshots.
        k(
            28,
            Some(ch(shift | cmd, '3')),
            true,
            "Save picture of screen as a file",
            BeforeApps,
        ),
        k(
            29,
            Some(ch(ctrl | shift | cmd, '3')),
            true,
            "Copy picture of screen to the clipboard",
            BeforeApps,
        ),
        k(
            30,
            Some(ch(shift | cmd, '4')),
            true,
            "Save picture of selected area as a file",
            BeforeApps,
        ),
        k(
            31,
            Some(ch(ctrl | shift | cmd, '4')),
            true,
            "Copy picture of selected area to the clipboard",
            BeforeApps,
        ),
        k(
            184,
            Some(ch(shift | cmd, '5')),
            true,
            "Screenshot and recording options",
            BeforeApps,
        ),
        k(
            181,
            Some(ch(shift | cmd, '6')),
            true,
            "Save picture of the Touch Bar as a file",
            BeforeApps,
        ),
        k(
            182,
            Some(ch(ctrl | shift | cmd, '6')),
            true,
            "Copy picture of the Touch Bar to the clipboard",
            BeforeApps,
        ),
        // Mission Control and Spaces.
        k(
            32,
            Some(named(ctrl, Up)),
            true,
            "Mission Control",
            BeforeApps,
        ),
        k(
            33,
            Some(named(ctrl, Down)),
            true,
            "Application windows",
            BeforeApps,
        ),
        k(
            36,
            Some(named(Modifiers::empty(), F11)),
            true,
            "Show Desktop",
            BeforeApps,
        ),
        k(
            79,
            Some(named(ctrl, Left)),
            true,
            "Move left a space",
            BeforeApps,
        ),
        k(
            81,
            Some(named(ctrl, Right)),
            true,
            "Move right a space",
            BeforeApps,
        ),
        // Only listed in System Settings once a second desktop exists, and
        // unchecked even then.
        k(
            118,
            Some(ch(ctrl, '1')),
            false,
            "Switch to Desktop 1",
            BeforeApps,
        ),
        k(
            119,
            Some(ch(ctrl, '2')),
            false,
            "Switch to Desktop 2",
            BeforeApps,
        ),
        k(
            120,
            Some(ch(ctrl, '3')),
            false,
            "Switch to Desktop 3",
            BeforeApps,
        ),
        k(
            121,
            Some(ch(ctrl, '4')),
            false,
            "Switch to Desktop 4",
            BeforeApps,
        ),
        // Spotlight.
        k(
            64,
            Some(named(cmd, Space)),
            true,
            "Show Spotlight search",
            BeforeApps,
        ),
        k(
            65,
            Some(named(opt | cmd, Space)),
            true,
            "Show Finder search window",
            BeforeApps,
        ),
        // Input sources. Shipped state depends on locale (off in the
        // English user template, on in zh_CN); the live table settles it.
        k(
            60,
            Some(named(ctrl, Space)),
            true,
            "Select the previous input source",
            BeforeApps,
        ),
        k(
            61,
            Some(named(ctrl | opt, Space)),
            true,
            "Select next source in Input menu",
            BeforeApps,
        ),
        // Dock, Help, and ids that ship without a key.
        k(
            52,
            Some(ch(opt | cmd, 'd')),
            true,
            "Turn Dock hiding on/off",
            BeforeApps,
        ),
        k(
            98,
            Some(ch(shift | cmd, '/')),
            true,
            "Show Help menu",
            StandardMenuItem,
        ),
        k(160, None, true, "Show Launchpad", BeforeApps),
        k(163, None, true, "Show Notification Center", BeforeApps),
        k(175, None, true, "Turn Do Not Disturb on/off", BeforeApps),
    ]
}

/// Shortcuts macOS registers without a plist id.
pub(super) fn builtin_shortcuts() -> Vec<BuiltinShortcut> {
    use NamedKey::*;
    use SystemDispatch::{BeforeApps, StandardMenuItem};
    let cmd = Modifiers::CMD;
    let shift = Modifiers::SHIFT;
    let ctrl = Modifiers::CTRL;
    let opt = Modifiers::OPT;
    let f = Modifiers::FN;
    let b = |combo, label, dispatch| BuiltinShortcut {
        combo,
        label,
        dispatch,
    };

    vec![
        // The window server owns these outright.
        b(named(cmd, Tab), "Switch to next app", BeforeApps),
        b(
            named(shift | cmd, Tab),
            "Switch to previous app",
            BeforeApps,
        ),
        b(named(opt | cmd, Escape), "Force Quit", BeforeApps),
        b(
            named(opt | shift | cmd, Escape),
            "Force Quit frontmost app",
            BeforeApps,
        ),
        b(
            named(opt | cmd, F5),
            "Show Accessibility controls",
            BeforeApps,
        ),
        b(ch(ctrl, '5'), "Switch to Desktop 5", BeforeApps),
        b(ch(ctrl, '6'), "Switch to Desktop 6", BeforeApps),
        b(ch(ctrl, '7'), "Switch to Desktop 7", BeforeApps),
        b(ch(ctrl, '8'), "Switch to Desktop 8", BeforeApps),
        b(ch(ctrl, '9'), "Switch to Desktop 9", BeforeApps),
        b(ch(ctrl, '0'), "Switch to Desktop 10", BeforeApps),
        b(ch(f, 'q'), "Quick Note", BeforeApps),
        b(ch(f, 'c'), "Show Control Center", BeforeApps),
        b(ch(f, 'n'), "Show Notification Center", BeforeApps),
        // Standard menu items AppKit adds to every app. The frontmost
        // app's menu dispatches them, so they never shadow an app.
        b(ch(cmd, 'm'), "Minimize", StandardMenuItem),
        b(ch(opt | cmd, 'm'), "Minimize All", StandardMenuItem),
        b(
            ch(shift | cmd, '`'),
            "Move focus to previous window",
            StandardMenuItem,
        ),
        b(
            named(ctrl | cmd, Space),
            "Emoji & Symbols",
            StandardMenuItem,
        ),
        b(ch(f, 'f'), "Enter or exit full screen", StandardMenuItem),
        b(ch(ctrl | f, 'c'), "Center", StandardMenuItem),
        b(ch(ctrl | f, 'f'), "Fill", StandardMenuItem),
        b(
            ch(ctrl | f, 'r'),
            "Return to Previous Size",
            StandardMenuItem,
        ),
        b(named(ctrl | f, Left), "Tile Left Half", StandardMenuItem),
        b(named(ctrl | f, Right), "Tile Right Half", StandardMenuItem),
        b(named(ctrl | f, Up), "Tile Top Half", StandardMenuItem),
        b(named(ctrl | f, Down), "Tile Bottom Half", StandardMenuItem),
    ]
}

pub(super) fn label_for(id: u32) -> Option<&'static str> {
    known_hotkeys()
        .into_iter()
        .find(|k| k.id == id)
        .map(|k| k.label)
}

/// Label and dispatch for a combo macOS registers without an id.
pub(super) fn builtin_for(combo: &KeyCombo) -> Option<BuiltinShortcut> {
    builtin_shortcuts().into_iter().find(|b| b.combo == *combo)
}

/// How macOS delivers the shortcut behind `id` / `combo`. Anything not
/// verified is assumed to be a standard menu item: that can under-report
/// a real system interception, but never invents a conflict.
pub(super) fn dispatch_for(id: Option<u32>, combo: &KeyCombo) -> SystemDispatch {
    if let Some(id) = id
        && let Some(k) = known_hotkeys().into_iter().find(|k| k.id == id)
    {
        return k.dispatch;
    }
    builtin_for(combo).map_or(SystemDispatch::StandardMenuItem, |b| b.dispatch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ids_are_unique_and_labelled() {
        let ids: Vec<u32> = known_hotkeys().iter().map(|k| k.id).collect();
        let mut dedup = ids.clone();
        dedup.sort_unstable();
        dedup.dedup();
        assert_eq!(ids.len(), dedup.len(), "duplicate id in known_hotkeys");
        assert!(known_hotkeys().iter().all(|k| !k.label.is_empty()));
        assert!(builtin_shortcuts().iter().all(|b| !b.label.is_empty()));
    }

    #[test]
    fn unknown_entries_default_to_standard_menu_items() {
        let unknown = ch(Modifiers::SHIFT | Modifiers::CMD, 'f');
        assert_eq!(
            dispatch_for(None, &unknown),
            SystemDispatch::StandardMenuItem
        );
        assert_eq!(
            dispatch_for(Some(9999), &unknown),
            SystemDispatch::StandardMenuItem
        );
    }

    #[test]
    fn interceptions_and_menu_defaults_are_told_apart() {
        assert_eq!(
            dispatch_for(Some(64), &named(Modifiers::CMD, NamedKey::Space)),
            SystemDispatch::BeforeApps
        );
        assert_eq!(
            dispatch_for(None, &named(Modifiers::CMD, NamedKey::Tab)),
            SystemDispatch::BeforeApps
        );
        assert_eq!(
            dispatch_for(None, &ch(Modifiers::CMD, 'm')),
            SystemDispatch::StandardMenuItem
        );
        assert_eq!(
            dispatch_for(Some(27), &ch(Modifiers::CMD, '`')),
            SystemDispatch::StandardMenuItem
        );
        assert_eq!(
            builtin_for(&ch(Modifiers::CMD, 'm')).unwrap().label,
            "Minimize"
        );
    }
}
