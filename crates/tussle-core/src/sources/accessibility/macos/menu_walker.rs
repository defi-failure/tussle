//! Walk an app's menu bars (main + status-bar extras) and harvest every
//! menu item with a key equivalent.

use accessibility_sys::{
    AXUIElementCreateApplication, AXUIElementRef, AXUIElementSetMessagingTimeout,
    kAXMenuBarAttribute, kAXMenuItemCmdCharAttribute, kAXMenuItemCmdModifiersAttribute,
    kAXTitleAttribute,
};

use crate::{Binding, BindingSource, Key, KeyCombo};

use super::ax::{copy_attribute, copy_children, copy_i64, copy_string};
use super::modifiers::decode_ax_modifiers;
use super::running_apps::RunningApp;

/// Hard cap on menu recursion depth to defend against pathological apps.
const MAX_MENU_DEPTH: usize = 16;

pub(super) fn walk_app_menus(app: &RunningApp, messaging_timeout: f32) -> Vec<Binding> {
    let started = std::time::Instant::now();
    let element = unsafe { AXUIElementCreateApplication(app.pid) };
    if element.is_null() {
        return Vec::new();
    }

    // Per-app timeout. Set on the application element propagates to
    // all child elements queried through it.
    if messaging_timeout > 0.0 {
        unsafe { AXUIElementSetMessagingTimeout(element, messaging_timeout) };
    }

    let mut bindings = Vec::new();

    // Main menu bar (visible when app is frontmost).
    if let Some(menu_bar) = copy_attribute(element, kAXMenuBarAttribute) {
        walk_menu(menu_bar, app, &[], 0, &mut bindings);
        unsafe { core_foundation::base::CFRelease(menu_bar as _) };
    }

    // Status-bar (NSStatusItem) dropdowns. Menubar-only apps like PixPin
    // expose their main shortcuts here, not on the regular menu bar.
    if let Some(extras) = copy_attribute(element, "AXExtrasMenuBar") {
        walk_menu(extras, app, &[], 0, &mut bindings);
        unsafe { core_foundation::base::CFRelease(extras as _) };
    }

    unsafe { core_foundation::base::CFRelease(element as _) };

    tracing::debug!(
        bundle = app.bundle_id.as_deref().unwrap_or("?"),
        bindings = bindings.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "walked app",
    );
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
        modifiers: decode_ax_modifiers(mask),
        key: Key::from_char(first_char),
    })
}
