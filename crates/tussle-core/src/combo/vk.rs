//! macOS virtual keycode → key tables.
//!
//! All values verified against `Carbon/HIToolbox.framework/Headers/Events.h`
//! (`kVK_*` constants). The two halves are split by output type:
//! [`vk_to_named`] for non-printable named keys (`kVK_Space`, `kVK_F1`, …),
//! [`vk_to_ansi_char`] for ANSI letter/digit/punctuation keys.

use super::key::NamedKey;

/// Map macOS virtual keycodes to `NamedKey`. `kVK_Delete` is Apple's name
/// for the main delete key — semantically Backspace on every other platform.
pub(crate) fn vk_to_named(vk: u16) -> Option<NamedKey> {
    let n = match vk {
        0x31 => NamedKey::Space,     // kVK_Space
        0x24 => NamedKey::Return,    // kVK_Return
        0x30 => NamedKey::Tab,       // kVK_Tab
        0x35 => NamedKey::Escape,    // kVK_Escape
        0x33 => NamedKey::Backspace, // kVK_Delete (Apple's main delete = Backspace)
        0x75 => NamedKey::Delete,    // kVK_ForwardDelete
        0x72 => NamedKey::Help,      // kVK_Help
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
        0x7E => NamedKey::Up,       // kVK_UpArrow
        0x7D => NamedKey::Down,     // kVK_DownArrow
        0x7B => NamedKey::Left,     // kVK_LeftArrow
        0x7C => NamedKey::Right,    // kVK_RightArrow
        0x74 => NamedKey::PageUp,   // kVK_PageUp
        0x79 => NamedKey::PageDown, // kVK_PageDown
        0x73 => NamedKey::Home,     // kVK_Home
        0x77 => NamedKey::End,      // kVK_End
        _ => return None,
    };
    Some(n)
}

/// Map ANSI-layout virtual keycodes to their lowercase character.
/// Source: `HIToolbox/Events.h` `kVK_ANSI_*` constants.
pub(crate) fn vk_to_ansi_char(vk: u16) -> Option<char> {
    Some(match vk {
        0x00 => 'a',
        0x01 => 's',
        0x02 => 'd',
        0x03 => 'f',
        0x04 => 'h',
        0x05 => 'g',
        0x06 => 'z',
        0x07 => 'x',
        0x08 => 'c',
        0x09 => 'v',
        0x0B => 'b',
        0x0C => 'q',
        0x0D => 'w',
        0x0E => 'e',
        0x0F => 'r',
        0x10 => 'y',
        0x11 => 't',
        0x12 => '1',
        0x13 => '2',
        0x14 => '3',
        0x15 => '4',
        0x17 => '5',
        0x16 => '6',
        0x1A => '7',
        0x1C => '8',
        0x19 => '9',
        0x1D => '0',
        0x18 => '=',
        0x1B => '-',
        0x21 => '[',
        0x1E => ']',
        0x27 => '\'',
        0x29 => ';',
        0x2A => '\\',
        0x2B => ',',
        0x2F => '.',
        0x2C => '/',
        0x32 => '`',
        0x1F => 'o',
        0x20 => 'u',
        0x22 => 'i',
        0x23 => 'p',
        0x25 => 'l',
        0x26 => 'j',
        0x28 => 'k',
        0x2D => 'n',
        0x2E => 'm',
        _ => return None,
    })
}
