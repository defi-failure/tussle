//! Parser for `~/Library/Preferences/com.apple.symbolichotkeys.plist`.
//!
//! The plist holds user customizations of macOS system shortcuts (Spotlight,
//! Mission Control, screenshots, ...). Each entry is a numeric ID mapping to
//! `{ enabled, value: { parameters: [char_code, virtual_keycode, mask], type } }`.
//! Defaults that the user has not overridden are NOT in this file — they are
//! hard-coded in macOS itself.

use std::path::Path;

use crate::{Binding, BindingSource, Key, KeyCombo, Modifiers, NamedKey, ScanError};

/// `parameters` array index for the printable character code, the macOS
/// virtual keycode, and the NSEvent modifier mask, respectively.
const PARAM_CHAR: usize = 0;
const PARAM_VK: usize = 1;
const PARAM_MASK: usize = 2;

/// Sentinel value Apple writes when a parameter slot is unset.
const UNSET: i64 = 65535;

/// NSEvent modifier flag bits. Defined in `<AppKit/NSEvent.h>`.
const NS_SHIFT: u64 = 0x0002_0000;
const NS_CTRL: u64 = 0x0004_0000;
const NS_OPT: u64 = 0x0008_0000;
const NS_CMD: u64 = 0x0010_0000;
const NS_FN: u64 = 0x0080_0000;

/// Parse a symbolichotkeys plist into the bindings it represents.
///
/// Disabled entries are filtered out. Entries lacking a `value` dict (which
/// means "use macOS default") are skipped at this layer because we cannot
/// know the default from this file alone.
pub fn scan(path: &Path) -> Result<Vec<Binding>, ScanError> {
    let bytes = std::fs::read(path).map_err(|source| ScanError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let value: plist::Value = plist::from_bytes(&bytes).map_err(|e| ScanError::Schema {
        path: path.to_path_buf(),
        message: format!("plist parse: {e}"),
    })?;

    let root = value.as_dictionary().ok_or_else(|| ScanError::Schema {
        path: path.to_path_buf(),
        message: "root is not a dictionary".into(),
    })?;

    let entries = root
        .get("AppleSymbolicHotKeys")
        .and_then(|v| v.as_dictionary())
        .ok_or_else(|| ScanError::Schema {
            path: path.to_path_buf(),
            message: "missing AppleSymbolicHotKeys dict".into(),
        })?;

    let mut bindings = Vec::new();

    for (id_str, entry) in entries {
        let Ok(id) = id_str.parse::<u32>() else {
            continue; // non-numeric keys aren't ours to handle
        };

        let Some(entry_dict) = entry.as_dictionary() else {
            continue;
        };

        let enabled = entry_dict
            .get("enabled")
            .and_then(|v| v.as_boolean())
            .unwrap_or(true);
        if !enabled {
            continue;
        }

        let Some(value_dict) = entry_dict.get("value").and_then(|v| v.as_dictionary()) else {
            // Enabled but no override → uses the macOS default, which lives
            // outside this file. Skip; future code can cross-reference a
            // bundled defaults table.
            continue;
        };

        let Some(params) = value_dict.get("parameters").and_then(|v| v.as_array()) else {
            continue;
        };
        if params.len() < 3 {
            continue;
        }

        let char_code = params[PARAM_CHAR].as_signed_integer().unwrap_or(UNSET);
        let vk = params[PARAM_VK].as_signed_integer().unwrap_or(UNSET);
        let mask = params[PARAM_MASK].as_signed_integer().unwrap_or(0);

        bindings.push(Binding {
            combo: KeyCombo {
                modifiers: decode_modifiers(mask as u64),
                key: decode_key(char_code, vk),
            },
            source: BindingSource::SystemSymbolicHotkey { id },
            label: label_for(id)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Symbolic hotkey #{id}")),
        });
    }

    Ok(bindings)
}

fn decode_modifiers(mask: u64) -> Modifiers {
    let mut m = Modifiers::empty();
    if mask & NS_CMD != 0 {
        m |= Modifiers::CMD;
    }
    if mask & NS_OPT != 0 {
        m |= Modifiers::OPT;
    }
    if mask & NS_CTRL != 0 {
        m |= Modifiers::CTRL;
    }
    if mask & NS_SHIFT != 0 {
        m |= Modifiers::SHIFT;
    }
    if mask & NS_FN != 0 {
        m |= Modifiers::FN;
    }
    m
}

fn decode_key(char_code: i64, vk: i64) -> Key {
    // Virtual keycode wins for keys with a canonical NamedKey, since the vk
    // is layout-independent while the char_code reflects the active layout.
    if vk != UNSET && (0..=u16::MAX as i64).contains(&vk) {
        if let Some(named) = vk_to_named(vk as u16) {
            return Key::Named(named);
        }
    }

    // Fall back to the printable character if Apple set one.
    if char_code != UNSET && (0..=u32::MAX as i64).contains(&char_code) {
        if let Some(c) = char::from_u32(char_code as u32) {
            if !c.is_control() {
                return Key::Char(c);
            }
        }
    }

    // Last resort: surface the raw vk so the caller can still see what was
    // bound, even if we don't have a name for it.
    if vk != UNSET && (0..=u16::MAX as i64).contains(&vk) {
        return Key::Virtual(vk as u16);
    }

    Key::Virtual(0)
}

/// Human-readable label for a known symbolic hotkey ID, or `None` if we don't
/// have a mapping yet. Labels track Apple's wording in System Settings →
/// Keyboard → Keyboard Shortcuts.
///
/// Coverage is partial; new IDs should be added as they show up in real
/// fixtures rather than guessed at.
fn label_for(id: u32) -> Option<&'static str> {
    Some(match id {
        // Keyboard navigation (Keyboard Access pane)
        7 => "Move focus to the menu bar",
        8 => "Move focus to the Dock",
        9 => "Move focus to the active or next window",
        10 => "Move focus to the window toolbar",
        11 => "Move focus to the floating window",
        12 => "Toggle keyboard access",
        13 => "Change the way Tab moves focus",
        27 => "Move focus to next window in application",
        51 => "Move focus to the window drawer",
        57 => "Move focus to the status menus",

        // Screenshots
        28 => "Save picture of screen as a file",
        29 => "Copy picture of screen to the clipboard",
        30 => "Save picture of selected area as a file",
        31 => "Copy picture of selected area to the clipboard",
        184 => "Screenshot and recording options",

        // Mission Control
        32 => "Mission Control",
        33 => "Application windows",
        36 => "Show Desktop",

        // Spotlight
        64 => "Show Spotlight search",
        65 => "Show Finder search window",

        // Input sources
        60 => "Select the previous input source",
        61 => "Select the next source in the Input menu",

        // Spaces (the duplicate IDs are the regular vs. modified-arrow forms)
        79 | 80 => "Move left a space",
        81 | 82 => "Move right a space",
        118 => "Switch to Desktop 1",
        119 => "Switch to Desktop 2",
        120 => "Switch to Desktop 3",
        121 => "Switch to Desktop 4",

        // Other system
        52 => "Toggle Dock hiding",
        59 => "Toggle VoiceOver",
        160 => "Show Launchpad",
        163 => "Show Notification Center",
        175 => "Toggle Do Not Disturb",

        // Touch Bar
        181 => "Save picture of the Touch Bar as a file",
        182 => "Copy picture of the Touch Bar to the clipboard",

        _ => return None,
    })
}

/// Map macOS virtual keycodes (from `<HIToolbox/Events.h>`) to `NamedKey`.
fn vk_to_named(vk: u16) -> Option<NamedKey> {
    let n = match vk {
        0x31 => NamedKey::Space,
        0x24 => NamedKey::Return,
        0x30 => NamedKey::Tab,
        0x35 => NamedKey::Escape,
        0x75 => NamedKey::Delete,
        0x33 => NamedKey::Backspace,
        0x7A => NamedKey::F1,
        0x78 => NamedKey::F2,
        0x63 => NamedKey::F3,
        0x76 => NamedKey::F4,
        0x60 => NamedKey::F5,
        0x61 => NamedKey::F6,
        0x62 => NamedKey::F7,
        0x64 => NamedKey::F8,
        0x65 => NamedKey::F9,
        0x6D => NamedKey::F10,
        0x67 => NamedKey::F11,
        0x6F => NamedKey::F12,
        0x69 => NamedKey::F13,
        0x6B => NamedKey::F14,
        0x71 => NamedKey::F15,
        0x6A => NamedKey::F16,
        0x40 => NamedKey::F17,
        0x4F => NamedKey::F18,
        0x50 => NamedKey::F19,
        0x5A => NamedKey::F20,
        0x7E => NamedKey::Up,
        0x7D => NamedKey::Down,
        0x7B => NamedKey::Left,
        0x7C => NamedKey::Right,
        0x74 => NamedKey::PageUp,
        0x79 => NamedKey::PageDown,
        0x73 => NamedKey::Home,
        0x77 => NamedKey::End,
        _ => return None,
    };
    Some(n)
}
