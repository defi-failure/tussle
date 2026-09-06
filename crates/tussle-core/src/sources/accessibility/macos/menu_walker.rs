//! Walk an app's menu bars (main + status-bar extras) and harvest every
//! menu item with a key equivalent.

use std::time::Duration;

use accessibility_sys::{
    AXUIElementCreateApplication, AXUIElementRef, AXUIElementSetMessagingTimeout,
    kAXMenuBarAttribute, kAXMenuItemCmdCharAttribute, kAXMenuItemCmdModifiersAttribute,
    kAXTitleAttribute,
};

use crate::{Binding, BindingSource, Key, KeyCombo, Modifiers, NamedKey};

use super::ax::{AxFailure, copy_attribute, copy_children, copy_i64, copy_string};
use super::modifiers::decode_ax_modifiers;
use super::running_apps::RunningApp;

/// Hard cap on menu recursion depth to defend against pathological apps.
const MAX_MENU_DEPTH: usize = 16;

/// A `CannotComplete` that took at least this share of the configured
/// timeout is a timeout; a faster one means the process has no
/// Accessibility server and is not worth retrying.
const TIMEOUT_SHARE: f32 = 0.8;

/// Threshold used when the caller left the macOS default timeout (about
/// 6s) in place.
const DEFAULT_TIMEOUT_THRESHOLD_SECS: f32 = 4.0;

/// Everything one walk of an app produced.
#[derive(Debug, Default)]
pub(super) struct WalkResult {
    pub(super) bindings: Vec<Binding>,
    /// The app stopped answering before the walk finished, so `bindings`
    /// is incomplete, possibly empty.
    pub(super) timed_out: bool,
}

pub(super) fn walk_app_menus(app: &RunningApp, messaging_timeout: f32) -> WalkResult {
    let started = std::time::Instant::now();
    let element = unsafe { AXUIElementCreateApplication(app.pid) };
    if element.is_null() {
        return WalkResult::default();
    }

    // Per-app timeout. Set on the application element propagates to
    // all child elements queried through it.
    if messaging_timeout > 0.0 {
        unsafe { AXUIElementSetMessagingTimeout(element, messaging_timeout) };
    }

    let mut walk = Walk {
        app,
        timeout: messaging_timeout,
        status_bar: false,
        result: WalkResult::default(),
    };

    // Main menu bar (visible when app is frontmost).
    match copy_attribute(element, kAXMenuBarAttribute) {
        Ok(menu_bar) => {
            walk.menu(menu_bar, &[], 0);
            unsafe { core_foundation::base::CFRelease(menu_bar as _) };
        }
        Err(failure) => {
            walk.stalled(failure);
        }
    }

    // Status-bar (NSStatusItem) dropdowns. Menubar-only apps like PixPin
    // expose their main shortcuts here, not on the regular menu bar, and
    // those shortcuts are global hotkeys in practice.
    if !walk.result.timed_out {
        walk.status_bar = true;
        match copy_attribute(element, "AXExtrasMenuBar") {
            Ok(extras) => {
                walk.menu(extras, &[], 0);
                unsafe { core_foundation::base::CFRelease(extras as _) };
            }
            Err(failure) => {
                walk.stalled(failure);
            }
        }
    }

    unsafe { core_foundation::base::CFRelease(element as _) };

    tracing::debug!(
        bundle = app.bundle_id.as_deref().unwrap_or("?"),
        bindings = walk.result.bindings.len(),
        timed_out = walk.result.timed_out,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "walked app",
    );
    walk.result
}

/// One walk in progress. Once a query times out the walk stops descending:
/// the app is not answering, and every further call would burn another
/// full timeout for nothing.
struct Walk<'a> {
    app: &'a RunningApp,
    timeout: f32,
    /// Whether the elements being visited hang off the status bar
    /// (`AXExtrasMenuBar`) rather than the main menu bar.
    status_bar: bool,
    result: WalkResult,
}

impl Walk<'_> {
    /// Record a failed query. Returns `true` when the walk must stop
    /// because the app is not answering in time.
    fn stalled(&mut self, failure: AxFailure) -> bool {
        match failure {
            AxFailure::CannotComplete(elapsed) if is_timeout(elapsed, self.timeout) => {
                self.result.timed_out = true;
                true
            }
            _ => false,
        }
    }

    /// Recursively walk a menu element. Each menu has child menu items;
    /// each menu item with a submenu has a child of type `AXMenu` whose
    /// children are the inner items.
    fn menu(&mut self, menu: AXUIElementRef, path: &[String], depth: usize) {
        if depth > MAX_MENU_DEPTH || self.result.timed_out {
            return;
        }

        let children = match copy_children(menu) {
            Ok(children) => children,
            Err(failure) => {
                self.stalled(failure);
                return;
            }
        };

        for i in 0..children.len() {
            if self.result.timed_out {
                return;
            }
            let child = unsafe { *children.get_unchecked(i) } as AXUIElementRef;
            self.item(child, path, depth);
        }
    }

    fn item(&mut self, item: AXUIElementRef, path: &[String], depth: usize) {
        let title = match copy_string(item, kAXTitleAttribute) {
            Ok(title) => title,
            Err(failure) => {
                if self.stalled(failure) {
                    return;
                }
                String::new()
            }
        };
        let mut new_path: Vec<String> = path.to_vec();
        if !title.is_empty() {
            new_path.push(title.clone());
        }

        match read_key_equivalent(item) {
            Ok(Some(combo)) => {
                let bundle_id = self.app.bundle_id.clone().unwrap_or_default();
                let app_name = self.app.app_name.clone();
                let menu_path = new_path.clone();
                let source = if self.status_bar && looks_like_global_hotkey(&combo) {
                    BindingSource::StatusMenuItem {
                        bundle_id,
                        app_name,
                        menu_path,
                    }
                } else {
                    BindingSource::AppMenuItem {
                        bundle_id,
                        app_name,
                        menu_path,
                    }
                };
                self.result.bindings.push(Binding {
                    combo,
                    source,
                    label: title.clone(),
                    enabled: true,
                });
            }
            Ok(None) => {}
            Err(failure) => {
                if self.stalled(failure) {
                    return;
                }
            }
        }

        // A menu item that opens a submenu has a child element of type
        // AXMenu whose children are the submenu's items.
        match copy_children(item) {
            Ok(grand) => {
                for j in 0..grand.len() {
                    if self.result.timed_out {
                        return;
                    }
                    let sub = unsafe { *grand.get_unchecked(j) } as AXUIElementRef;
                    self.menu(sub, &new_path, depth + 1);
                }
            }
            Err(failure) => {
                self.stalled(failure);
            }
        }
    }
}

/// Whether a shortcut shown in a status-bar menu is plausibly a global
/// hotkey rather than an ordinary menu item.
///
/// Status-bar menus mix both: PixPin lists its global ⌃1 there, Surge
/// lists a perfectly ordinary ⌘C. Nothing in the Accessibility API tells
/// them apart, so this goes by convention: global hotkeys avoid the
/// plain ⌘-plus-key shapes that every app already uses in its menus and
/// reach for ⌃, fn, ⌘-less combos or function keys instead. Erring
/// towards "ordinary" is deliberate; a missed global hotkey affects one
/// verdict, a false one floods `conflicts` with nonsense.
fn looks_like_global_hotkey(combo: &KeyCombo) -> bool {
    let m = combo.modifiers;
    !m.contains(Modifiers::CMD)
        || m.contains(Modifiers::CTRL)
        || m.contains(Modifiers::FN)
        || matches!(combo.key, Key::Named(k) if is_function_key(k))
}

fn is_function_key(key: NamedKey) -> bool {
    use NamedKey::*;
    matches!(
        key,
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
}

/// Whether a `CannotComplete` that took `elapsed` was the messaging
/// timeout expiring, as opposed to an immediate refusal.
fn is_timeout(elapsed: Duration, timeout: f32) -> bool {
    let threshold = if timeout > 0.0 {
        timeout * TIMEOUT_SHARE
    } else {
        DEFAULT_TIMEOUT_THRESHOLD_SECS
    };
    elapsed.as_secs_f32() >= threshold
}

/// `Ok(None)` when the item simply has no key equivalent, which is the
/// common case.
fn read_key_equivalent(item: AXUIElementRef) -> Result<Option<KeyCombo>, AxFailure> {
    let ch = match copy_string(item, kAXMenuItemCmdCharAttribute) {
        Ok(ch) => ch,
        Err(AxFailure::Other) => return Ok(None),
        Err(e) => return Err(e),
    };
    let Some(first_char) = ch.chars().next() else {
        return Ok(None);
    };
    let mask = match copy_i64(item, kAXMenuItemCmdModifiersAttribute) {
        Ok(mask) => mask,
        Err(AxFailure::Other) => 0,
        Err(e) => return Err(e),
    };
    Ok(Some(KeyCombo {
        modifiers: decode_ax_modifiers(mask),
        key: Key::from_char(first_char),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_cannot_complete_is_a_timeout_and_fast_one_is_not() {
        assert!(is_timeout(Duration::from_millis(900), 1.0));
        assert!(is_timeout(Duration::from_millis(1200), 1.0));
        assert!(!is_timeout(Duration::from_millis(5), 1.0));
        assert!(!is_timeout(Duration::from_millis(700), 1.0));
    }

    #[test]
    fn ordinary_command_shortcuts_are_not_global_hotkeys() {
        let combo = |m, k| KeyCombo {
            modifiers: m,
            key: k,
        };
        // Surge / OrbStack style status menus: plain menu items.
        assert!(!looks_like_global_hotkey(&combo(
            Modifiers::CMD,
            Key::Char('c')
        )));
        assert!(!looks_like_global_hotkey(&combo(
            Modifiers::CMD,
            Key::Char(',')
        )));
        assert!(!looks_like_global_hotkey(&combo(
            Modifiers::CMD | Modifiers::OPT,
            Key::Char('c')
        )));
        assert!(!looks_like_global_hotkey(&combo(
            Modifiers::CMD | Modifiers::SHIFT,
            Key::Char('4')
        )));
        // PixPin style: control-based, fn-based, ⌘-less, or function keys.
        assert!(looks_like_global_hotkey(&combo(
            Modifiers::CTRL,
            Key::Char('1')
        )));
        assert!(looks_like_global_hotkey(&combo(
            Modifiers::CTRL | Modifiers::CMD,
            Key::Named(NamedKey::Space)
        )));
        assert!(looks_like_global_hotkey(&combo(
            Modifiers::FN,
            Key::Char('a')
        )));
        assert!(looks_like_global_hotkey(&combo(
            Modifiers::OPT,
            Key::Named(NamedKey::Space)
        )));
        assert!(looks_like_global_hotkey(&combo(
            Modifiers::CMD,
            Key::Named(NamedKey::F5)
        )));
    }

    #[test]
    fn system_default_timeout_uses_fixed_threshold() {
        assert!(is_timeout(Duration::from_secs(5), 0.0));
        assert!(!is_timeout(Duration::from_millis(500), 0.0));
    }
}
