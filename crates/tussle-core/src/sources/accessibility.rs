//! Per-app menu shortcut enumeration via the macOS Accessibility API.
//!
//! Walks every running app's main menu bar and status items using
//! `AXUIElement` queries, extracting any menu item with a key equivalent.
//! Each match becomes a `Binding` with `BindingSource::AppMenuItem`.
//!
//! Requires the host process to have Accessibility permission
//! (System Settings → Privacy & Security → Accessibility). The first call
//! that exercises an `AX*` API triggers macOS's permission prompt.

use crate::{Binding, ScanError};

use super::Source;

/// Source backed by the macOS Accessibility API.
///
/// Has no configuration (paths come from the OS at scan time), so it's a
/// unit struct — `Accessibility` is the entire value.
#[derive(Debug, Clone, Copy, Default)]
pub struct Accessibility;

impl Source for Accessibility {
    fn name(&self) -> &'static str {
        "accessibility"
    }

    fn scan(&self) -> Result<Vec<Binding>, ScanError> {
        #[cfg(target_os = "macos")]
        {
            platform::scan()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(Vec::new())
        }
    }
}

/// Whether the host process currently has Accessibility permission.
///
/// On non-macOS platforms always returns `true` (no permission concept).
pub fn is_trusted() -> bool {
    #[cfg(target_os = "macos")]
    {
        platform::is_trusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::ffi::c_void;
    use std::ptr;

    use accessibility_sys::{
        kAXChildrenAttribute, kAXErrorSuccess, kAXMenuBarAttribute, kAXMenuItemCmdCharAttribute,
        kAXMenuItemCmdModifiersAttribute, kAXTitleAttribute, AXError, AXUIElementCopyAttributeValue,
        AXUIElementCreateApplication, AXUIElementRef,
    };
    use core_foundation::array::CFArray;
    use core_foundation::base::{CFTypeRef, TCFType};
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use objc2_app_kit::NSWorkspace;

    use crate::{Binding, BindingSource, Key, KeyCombo, Modifiers, ScanError};

    /// Hard cap on menu recursion depth to defend against pathological apps.
    const MAX_MENU_DEPTH: usize = 16;

    pub fn scan() -> Result<Vec<Binding>, ScanError> {
        if !is_trusted() {
            return Ok(Vec::new());
        }

        let mut bindings = Vec::new();
        for app in list_running_apps() {
            bindings.extend(walk_app_menus(&app));
        }
        Ok(bindings)
    }

    pub fn is_trusted() -> bool {
        // Safety: AXIsProcessTrusted is thread-safe and has no preconditions.
        unsafe { accessibility_sys::AXIsProcessTrusted() }
    }

    struct RunningApp {
        pid: i32,
        bundle_id: Option<String>,
        app_name: Option<String>,
    }

    fn list_running_apps() -> Vec<RunningApp> {
        let workspace = NSWorkspace::sharedWorkspace();
        let apps = workspace.runningApplications();
        let mut out = Vec::with_capacity(apps.len());
        for app in apps.iter() {
            out.push(RunningApp {
                pid: app.processIdentifier(),
                bundle_id: app.bundleIdentifier().map(|s| s.to_string()),
                app_name: app.localizedName().map(|s| s.to_string()),
            });
        }
        out
    }

    fn walk_app_menus(app: &RunningApp) -> Vec<Binding> {
        let element = unsafe { AXUIElementCreateApplication(app.pid) };
        if element.is_null() {
            return Vec::new();
        }

        let mut bindings = Vec::new();
        if let Some(menu_bar) = copy_attribute(element, kAXMenuBarAttribute) {
            walk_menu(menu_bar, app, &[], 0, &mut bindings);
            unsafe { core_foundation::base::CFRelease(menu_bar as _) };
        }

        unsafe { core_foundation::base::CFRelease(element as _) };
        bindings
    }

    /// Recursively walk a menu element. Each menu has child menu items;
    /// each menu item with a submenu has a child of type `AXMenu` whose
    /// children are the inner items.
    fn walk_menu(
        menu: AXUIElementRef,
        app: &RunningApp,
        path: &[String],
        depth: usize,
        out: &mut Vec<Binding>,
    ) {
        if depth > MAX_MENU_DEPTH {
            return;
        }

        let Some(children) = copy_children(menu) else {
            return;
        };

        for i in 0..children.len() {
            let child = unsafe { *children.get_unchecked(i) } as AXUIElementRef;
            visit_item(child, app, path, depth, out);
        }
    }

    fn visit_item(
        item: AXUIElementRef,
        app: &RunningApp,
        path: &[String],
        depth: usize,
        out: &mut Vec<Binding>,
    ) {
        let title = copy_string(item, kAXTitleAttribute).unwrap_or_default();
        let mut new_path: Vec<String> = path.to_vec();
        if !title.is_empty() {
            new_path.push(title.clone());
        }

        if let Some(combo) = read_key_equivalent(item) {
            out.push(Binding {
                combo,
                source: BindingSource::AppMenuItem {
                    bundle_id: app.bundle_id.clone().unwrap_or_default(),
                    app_name: app.app_name.clone(),
                    menu_path: new_path.clone(),
                },
                label: title.clone(),
            });
        }

        // A menu item that opens a submenu has a child element of type
        // AXMenu whose children are the submenu's items.
        if let Some(grand) = copy_children(item) {
            for j in 0..grand.len() {
                let sub = unsafe { *grand.get_unchecked(j) } as AXUIElementRef;
                walk_menu(sub, app, &new_path, depth + 1, out);
            }
        }
    }

    fn read_key_equivalent(item: AXUIElementRef) -> Option<KeyCombo> {
        let ch = copy_string(item, kAXMenuItemCmdCharAttribute)?;
        let first_char = ch.chars().next()?;
        let mask = copy_i64(item, kAXMenuItemCmdModifiersAttribute).unwrap_or(0);
        Some(KeyCombo {
            modifiers: decode_ax_modifiers(mask) | Modifiers::CMD,
            key: Key::Char(first_char.to_ascii_lowercase()),
        })
    }

    /// Accessibility's `AXMenuItemCmdModifiers` is a small integer where the
    /// command key is implicit; bits 1/2/4/8 add Shift/Option/Control/none.
    /// We translate that into our `Modifiers` set (Cmd is added by the
    /// caller because it's always implied for menu shortcuts).
    fn decode_ax_modifiers(mask: i64) -> Modifiers {
        let mut m = Modifiers::empty();
        if mask & 0x01 != 0 {
            m |= Modifiers::SHIFT;
        }
        if mask & 0x02 != 0 {
            m |= Modifiers::OPT;
        }
        if mask & 0x04 != 0 {
            m |= Modifiers::CTRL;
        }
        // Bit 0x08 means "no command key" — i.e. the shortcut does NOT
        // include Cmd. The caller decides whether to add CMD by default.
        if mask & 0x08 != 0 {
            m.remove(Modifiers::CMD);
        }
        m
    }

    fn copy_attribute(element: AXUIElementRef, attribute: &str) -> Option<AXUIElementRef> {
        let attr = CFString::new(attribute);
        let mut value: CFTypeRef = ptr::null();
        let err: AXError = unsafe {
            AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value)
        };
        if err != kAXErrorSuccess || value.is_null() {
            return None;
        }
        Some(value as AXUIElementRef)
    }

    fn copy_children(element: AXUIElementRef) -> Option<CFArray<*const c_void>> {
        let attr = CFString::new(kAXChildrenAttribute);
        let mut value: CFTypeRef = ptr::null();
        let err = unsafe {
            AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value)
        };
        if err != kAXErrorSuccess || value.is_null() {
            return None;
        }
        Some(unsafe { CFArray::wrap_under_create_rule(value as _) })
    }

    fn copy_string(element: AXUIElementRef, attribute: &str) -> Option<String> {
        let attr = CFString::new(attribute);
        let mut value: CFTypeRef = ptr::null();
        let err = unsafe {
            AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value)
        };
        if err != kAXErrorSuccess || value.is_null() {
            return None;
        }
        let s = unsafe { CFString::wrap_under_create_rule(value as _) };
        Some(s.to_string())
    }

    fn copy_i64(element: AXUIElementRef, attribute: &str) -> Option<i64> {
        let attr = CFString::new(attribute);
        let mut value: CFTypeRef = ptr::null();
        let err = unsafe {
            AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value)
        };
        if err != kAXErrorSuccess || value.is_null() {
            return None;
        }
        let n = unsafe { CFNumber::wrap_under_create_rule(value as _) };
        n.to_i64()
    }
}
