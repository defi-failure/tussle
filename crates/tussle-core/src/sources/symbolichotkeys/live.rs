//! The system's own view of its symbolic hotkeys, read through Carbon's
//! `CopySymbolicHotKeys()`.
//!
//! `com.apple.symbolichotkeys.plist` only records what the user changed;
//! the built-in defaults live inside macOS and differ by locale (the
//! zh_CN user template ships input-source switching on, the English one
//! off). `CopySymbolicHotKeys()` returns the effective table after macOS
//! has merged defaults and preferences, including shortcuts that never
//! get a plist id at all (⌘Tab, ⌥⌘Esc). Entries carry a virtual keycode,
//! a Carbon modifier mask and an enabled flag, but no id and no label.

use crate::{Key, KeyCombo, Modifiers, NamedKey};

/// One row of the live table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveHotkey {
    /// Virtual keycode; `NO_KEY` when the shortcut has no key assigned.
    pub vk: u16,
    /// Carbon modifier mask: `cmdKey`, `shiftKey`, `optionKey`,
    /// `controlKey` and the fn bit.
    pub carbon_modifiers: u32,
    pub enabled: bool,
}

/// Placeholder keycode for entries without a key.
pub const NO_KEY: u16 = 0xFFFF;

// Carbon modifier bits, from `HIToolbox/Events.h`:
//   cmdKey     = 1 << 8
//   shiftKey   = 1 << 9
//   alphaLock  = 1 << 10 (never set in this table)
//   optionKey  = 1 << 11
//   controlKey = 1 << 12
//   fn         = 1 << 17 (kEventKeyModifierFnMask)
const CARBON_CMD: u32 = 1 << 8;
const CARBON_SHIFT: u32 = 1 << 9;
const CARBON_OPT: u32 = 1 << 11;
const CARBON_CTRL: u32 = 1 << 12;
const CARBON_FN: u32 = 1 << 17;

impl LiveHotkey {
    /// The combo this entry is bound to, or `None` when it has no key.
    ///
    /// The fn bit is dropped on F1–F20: macOS sets it there to mean "the
    /// function key, not the media key printed on it", which is how users
    /// write `ctrl+f2`. Everywhere else fn is a real modifier (fn+Q opens
    /// Quick Note) and is kept.
    pub fn combo(&self) -> Option<KeyCombo> {
        if self.vk == NO_KEY {
            return None;
        }
        let key = Key::from_vk(self.vk);
        let mut modifiers = Modifiers::empty();
        let m = self.carbon_modifiers;
        if m & CARBON_CMD != 0 {
            modifiers |= Modifiers::CMD;
        }
        if m & CARBON_SHIFT != 0 {
            modifiers |= Modifiers::SHIFT;
        }
        if m & CARBON_OPT != 0 {
            modifiers |= Modifiers::OPT;
        }
        if m & CARBON_CTRL != 0 {
            modifiers |= Modifiers::CTRL;
        }
        if m & CARBON_FN != 0 && !is_function_key(&key) {
            modifiers |= Modifiers::FN;
        }
        Some(KeyCombo { modifiers, key })
    }
}

fn is_function_key(key: &Key) -> bool {
    use NamedKey::*;
    matches!(
        key,
        Key::Named(
            F1 | F2
                | F3
                | F4
                | F5
                | F6
                | F7
                | F8
                | F9
                | F10
                | F11
                | F12
                | F13
                | F14
                | F15
                | F16
                | F17
                | F18
                | F19
                | F20
        )
    )
}

#[cfg(target_os = "macos")]
mod ffi {
    use core_foundation::array::{CFArray, CFArrayRef};
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;

    use super::{LiveHotkey, NO_KEY};

    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn CopySymbolicHotKeys(out_hot_keys: *mut CFArrayRef) -> i32;
    }

    /// Ask HIToolbox for the effective symbolic hotkey table.
    pub(super) fn copy_symbolic_hotkeys() -> Result<Vec<LiveHotkey>, String> {
        let mut raw: CFArrayRef = std::ptr::null();
        // SAFETY: CopySymbolicHotKeys writes a +1 retained CFArray of
        // CFDictionaries into `raw` on success and needs no permissions.
        let status = unsafe { CopySymbolicHotKeys(&mut raw) };
        if status != 0 || raw.is_null() {
            return Err(format!("CopySymbolicHotKeys returned status {status}"));
        }
        // SAFETY: ownership of the +1 reference passes to the wrapper.
        let table: CFArray<CFDictionary<CFString, CFType>> =
            unsafe { CFArray::wrap_under_create_rule(raw) };

        let key_code = CFString::from_static_string("kHISymbolicHotKeyCode");
        let key_modifiers = CFString::from_static_string("kHISymbolicHotKeyModifiers");
        let key_enabled = CFString::from_static_string("kHISymbolicHotKeyEnabled");

        let mut out = Vec::with_capacity(table.len() as usize);
        for entry in table.iter() {
            let number = |key: &CFString| {
                entry
                    .find(key)
                    .and_then(|v| v.downcast::<CFNumber>())
                    .and_then(|n| n.to_i64())
            };
            let vk = number(&key_code)
                .filter(|v| (0..=i64::from(NO_KEY)).contains(v))
                .map_or(NO_KEY, |v| v as u16);
            let carbon_modifiers = number(&key_modifiers)
                .filter(|m| (0..=i64::from(u32::MAX)).contains(m))
                .map_or(0, |m| m as u32);
            let enabled = entry
                .find(&key_enabled)
                .and_then(|v| v.downcast::<CFBoolean>())
                .is_some_and(bool::from);
            out.push(LiveHotkey {
                vk,
                carbon_modifiers,
                enabled,
            });
        }
        Ok(out)
    }
}

/// The live table, or an error message when it cannot be read (always
/// off macOS).
pub fn read_system_table() -> Result<Vec<LiveHotkey>, String> {
    #[cfg(target_os = "macos")]
    {
        ffi::copy_symbolic_hotkeys()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("symbolic hotkeys are a macOS facility".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(vk: u16, mods: u32) -> LiveHotkey {
        LiveHotkey {
            vk,
            carbon_modifiers: mods,
            enabled: true,
        }
    }

    #[test]
    fn decodes_plain_carbon_modifiers() {
        // ⌥⌘Esc (Force Quit) and ⌘Tab as they appear in the table.
        assert_eq!(
            row(0x35, 0x900).combo(),
            Some(KeyCombo {
                modifiers: Modifiers::OPT | Modifiers::CMD,
                key: Key::Named(NamedKey::Escape),
            })
        );
        assert_eq!(
            row(0x30, 0x100).combo(),
            Some(KeyCombo {
                modifiers: Modifiers::CMD,
                key: Key::Named(NamedKey::Tab),
            })
        );
        assert_eq!(
            row(0x12, 0x1000).combo().map(|c| c.to_string()),
            Some("ctrl+1".into())
        );
    }

    #[test]
    fn fn_bit_is_dropped_on_function_keys_only() {
        // fn+ctrl+F2 is how macOS registers "Move focus to the menu bar";
        // users write ctrl+f2.
        assert_eq!(
            row(0x78, 0x21000).combo().map(|c| c.to_string()),
            Some("ctrl+f2".into())
        );
        // F11 alone (Show Desktop).
        assert_eq!(
            row(0x67, 0x20000).combo().map(|c| c.to_string()),
            Some("f11".into())
        );
        // fn+Q (Quick Note) and fn+Left keep fn: it is a real modifier there.
        assert_eq!(
            row(0x0C, 0x20000).combo().map(|c| c.to_string()),
            Some("fn+q".into())
        );
        assert_eq!(
            row(0x7B, 0x20000).combo().map(|c| c.to_string()),
            Some("fn+left".into())
        );
    }

    #[test]
    fn entries_without_a_key_have_no_combo() {
        assert_eq!(row(NO_KEY, 0).combo(), None);
    }
}
