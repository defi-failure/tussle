use crate::KeyCombo;

/// A discovered keyboard shortcut binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The key combination this binding is registered for.
    pub combo: KeyCombo,
    /// Where the binding was discovered.
    pub source: BindingSource,
    /// Human-readable label (e.g. "Show Spotlight search" or
    /// "Open Brave (Raycast extension)").
    pub label: String,
    /// Whether the binding currently fires. Sources that can see an entry
    /// the user switched off (a disabled system shortcut, a disabled
    /// launcher rule) report it with `enabled = false`, so a free combo can
    /// still be explained without being counted as taken.
    pub enabled: bool,
}

/// The system, app, or launcher that owns a binding.
///
/// New variants are added as parsers come online; keep this enum extensible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingSource {
    /// A macOS system shortcut sourced from
    /// `~/Library/Preferences/com.apple.symbolichotkeys.plist`.
    ///
    /// `id` is the symbolic hotkey numeric identifier (e.g. 64 = Spotlight)
    /// when the entry is one the user can see in System Settings; `None`
    /// for shortcuts macOS registers without exposing an id, such as ⌘Tab.
    /// `dispatch` says whether macOS grabs the key before apps or merely
    /// offers it as a standard menu item.
    SystemSymbolicHotkey {
        id: Option<u32>,
        dispatch: SystemDispatch,
    },

    /// A per-app menu-item override stored in the app's
    /// `NSUserKeyEquivalents` dictionary at
    /// `~/Library/Preferences/<bundle_id>.plist`.
    AppMenuOverride {
        /// The app's bundle identifier (e.g. `"com.apple.TextEdit"`).
        bundle_id: String,
        /// The menu item the user remapped (e.g. `"New"`, `"Open Recent"`).
        menu_item: String,
    },

    /// A menu item declared by a running app, discovered via the macOS
    /// Accessibility API. Distinct from `AppMenuOverride` (which is the
    /// user override layer); this is the app's own menu binding.
    AppMenuItem {
        /// The app's bundle identifier (e.g. `"com.apple.Safari"`).
        bundle_id: String,
        /// Localized app name as macOS reports it (e.g. `"Safari"`),
        /// when available.
        app_name: Option<String>,
        /// Menu hierarchy path to the item, top-level first
        /// (e.g. `["File", "New Window"]`).
        menu_path: Vec<String>,
    },
    /// A key equivalent shown in an app's status-bar (`NSStatusItem`)
    /// menu that looks like a global hotkey, discovered via the
    /// Accessibility API.
    ///
    /// A menu's key equivalents only fire while that menu is open, so a
    /// ⌃1 displayed there is useful to the user only because the app also
    /// registered it globally; menubar apps do exactly that. The
    /// registration itself is invisible to the Accessibility API, so this
    /// is a judgement by shape: status-bar shortcuts of the ordinary
    /// ⌘-plus-key kind (⌘C, ⌘Q, ⌘,) stay [`AppMenuItem`](Self::AppMenuItem).
    StatusMenuItem {
        /// The app's bundle identifier.
        bundle_id: String,
        /// Localized app name as macOS reports it, when available.
        app_name: Option<String>,
        /// Menu hierarchy path to the item, top-level first.
        menu_path: Vec<String>,
    },
}

/// How macOS delivers one of its own shortcuts.
///
/// The system hotkey table mixes two very different things. Spotlight,
/// Mission Control or ⌘Tab are handled by the window server before any
/// app sees the key. Minimize, Fill, Center or Emoji & Symbols are
/// standard menu items that AppKit adds to every app's menus; the
/// frontmost app's own menu handles them, and an app that binds the same
/// combo to something else simply wins. Only the first kind can shadow
/// an app's shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemDispatch {
    /// The window server handles it before any app sees the key.
    BeforeApps,
    /// Offered to every app as a standard menu item; the frontmost app
    /// dispatches it like any other menu shortcut.
    StandardMenuItem,
}

/// Where in macOS's keyboard pipeline a binding intercepts the keystroke.
///
/// Ordered from first to last. When two enabled global bindings claim the
/// same combo, the one on the earlier layer sees the event first and normally
/// wins; the later one never fires. The order follows how macOS delivers
/// key events: a virtual keyboard driver rewrites keys before the OS sees
/// them, a session event tap runs before the window server dispatches
/// system shortcuts, system shortcuts beat process-registered global
/// hotkeys, and an app's menu items only see whatever is left while that
/// app is frontmost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    /// Virtual keyboard driver (Karabiner-Elements). Rewrites keys before
    /// macOS sees them.
    Driver,
    /// Session-level `CGEventTap` (skhd, BetterTouchTool, Keyboard Maestro).
    EventTap,
    /// macOS system shortcuts from `com.apple.symbolichotkeys` (Spotlight,
    /// Mission Control, screenshots, ...).
    System,
    /// Process-registered global hotkeys via `RegisterEventHotKey` (Raycast,
    /// Alfred, Hammerspoon, ...).
    GlobalHotkey,
    /// A menu item of an application, including user overrides of one.
    /// Fires only while that app is frontmost.
    AppMenu,
}

/// When a binding can fire at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Regardless of which app is frontmost.
    Global,
    /// Only while the owning app is frontmost.
    App,
}

impl Layer {
    /// Whether bindings on this layer fire globally or only in their app.
    pub fn scope(self) -> Scope {
        match self {
            Layer::AppMenu => Scope::App,
            _ => Scope::Global,
        }
    }

    /// Stable lowercase identifier, for output and filters.
    pub fn name(self) -> &'static str {
        match self {
            Layer::Driver => "driver",
            Layer::EventTap => "event-tap",
            Layer::System => "system",
            Layer::GlobalHotkey => "global-hotkey",
            Layer::AppMenu => "app-menu",
        }
    }
}

impl BindingSource {
    /// The pipeline layer this binding lives on. Decides who wins when two
    /// global bindings share a combo.
    pub fn layer(&self) -> Layer {
        match self {
            BindingSource::SystemSymbolicHotkey {
                dispatch: SystemDispatch::BeforeApps,
                ..
            } => Layer::System,
            BindingSource::SystemSymbolicHotkey {
                dispatch: SystemDispatch::StandardMenuItem,
                ..
            } => Layer::AppMenu,
            BindingSource::StatusMenuItem { .. } => Layer::GlobalHotkey,
            BindingSource::AppMenuOverride { .. } | BindingSource::AppMenuItem { .. } => {
                Layer::AppMenu
            }
        }
    }

    /// Shorthand for `self.layer().scope()`.
    pub fn scope(&self) -> Scope {
        self.layer().scope()
    }

    /// Short human-readable identifier for whoever owns this binding —
    /// answers "who took this keystroke?" not "what does it do?".
    pub fn owner(&self) -> &str {
        match self {
            BindingSource::SystemSymbolicHotkey { .. } => "macOS",
            BindingSource::AppMenuOverride { bundle_id, .. } => bundle_id,
            BindingSource::AppMenuItem {
                app_name,
                bundle_id,
                ..
            }
            | BindingSource::StatusMenuItem {
                app_name,
                bundle_id,
                ..
            } => app_name.as_deref().unwrap_or(bundle_id),
        }
    }

    /// The owning app's bundle identifier, when there is one. `None` for
    /// system-level entries (e.g. SymbolicHotkey). Useful for filters that
    /// want to match by reverse-DNS id (`com.apple.finder`) regardless of
    /// the localized display name (`访达` on Chinese macOS, `Finder` on
    /// English).
    pub fn bundle_id(&self) -> Option<&str> {
        match self {
            BindingSource::SystemSymbolicHotkey { .. } => None,
            BindingSource::AppMenuOverride { bundle_id, .. } => Some(bundle_id),
            BindingSource::AppMenuItem { bundle_id, .. }
            | BindingSource::StatusMenuItem { bundle_id, .. } => Some(bundle_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_for_symbolic_hotkey_is_macos() {
        let s = BindingSource::SystemSymbolicHotkey {
            id: Some(64),
            dispatch: SystemDispatch::BeforeApps,
        };
        assert_eq!(s.owner(), "macOS");
    }

    #[test]
    fn owner_for_app_menu_override_is_bundle_id() {
        let s = BindingSource::AppMenuOverride {
            bundle_id: "com.apple.TextEdit".into(),
            menu_item: "New".into(),
        };
        assert_eq!(s.owner(), "com.apple.TextEdit");
    }

    #[test]
    fn owner_for_app_menu_item_prefers_app_name() {
        let s = BindingSource::AppMenuItem {
            bundle_id: "com.apple.Safari".into(),
            app_name: Some("Safari".into()),
            menu_path: vec!["File".into(), "New Window".into()],
        };
        assert_eq!(s.owner(), "Safari");
    }

    #[test]
    fn owner_for_app_menu_item_falls_back_to_bundle_id() {
        let s = BindingSource::AppMenuItem {
            bundle_id: "com.example.unknown".into(),
            app_name: None,
            menu_path: vec!["Edit".into()],
        };
        assert_eq!(s.owner(), "com.example.unknown");
    }

    #[test]
    fn bundle_id_is_none_for_system_hotkey() {
        let s = BindingSource::SystemSymbolicHotkey {
            id: Some(64),
            dispatch: SystemDispatch::BeforeApps,
        };
        assert!(s.bundle_id().is_none());
    }

    #[test]
    fn bundle_id_returns_id_for_app_sources() {
        let menu = BindingSource::AppMenuItem {
            bundle_id: "com.apple.finder".into(),
            app_name: Some("访达".into()),
            menu_path: vec![],
        };
        assert_eq!(menu.bundle_id(), Some("com.apple.finder"));

        let override_ = BindingSource::AppMenuOverride {
            bundle_id: "com.apple.TextEdit".into(),
            menu_item: "New".into(),
        };
        assert_eq!(override_.bundle_id(), Some("com.apple.TextEdit"));
    }

    #[test]
    fn layers_are_ordered_first_to_last() {
        assert!(Layer::Driver < Layer::EventTap);
        assert!(Layer::EventTap < Layer::System);
        assert!(Layer::System < Layer::GlobalHotkey);
        assert!(Layer::GlobalHotkey < Layer::AppMenu);
    }

    #[test]
    fn only_app_menu_layer_is_app_scoped() {
        assert_eq!(Layer::AppMenu.scope(), Scope::App);
        for layer in [
            Layer::Driver,
            Layer::EventTap,
            Layer::System,
            Layer::GlobalHotkey,
        ] {
            assert_eq!(layer.scope(), Scope::Global, "{layer:?}");
        }
    }

    #[test]
    fn standard_menu_defaults_are_app_scoped() {
        let minimize = BindingSource::SystemSymbolicHotkey {
            id: None,
            dispatch: SystemDispatch::StandardMenuItem,
        };
        assert_eq!(minimize.layer(), Layer::AppMenu);
        assert_eq!(minimize.scope(), Scope::App);
        assert_eq!(minimize.owner(), "macOS");
    }

    #[test]
    fn sources_map_to_layers() {
        assert_eq!(
            BindingSource::SystemSymbolicHotkey {
                id: Some(64),
                dispatch: SystemDispatch::BeforeApps,
            }
            .layer(),
            Layer::System
        );
        let override_ = BindingSource::AppMenuOverride {
            bundle_id: "com.apple.TextEdit".into(),
            menu_item: "New".into(),
        };
        assert_eq!(override_.layer(), Layer::AppMenu);
        assert_eq!(override_.scope(), Scope::App);
        let item = BindingSource::AppMenuItem {
            bundle_id: "com.apple.Safari".into(),
            app_name: None,
            menu_path: vec![],
        };
        assert_eq!(item.layer(), Layer::AppMenu);
        let status = BindingSource::StatusMenuItem {
            bundle_id: "com.pixpin.PixPin".into(),
            app_name: Some("PixPin".into()),
            menu_path: vec!["截图".into()],
        };
        assert_eq!(status.layer(), Layer::GlobalHotkey);
        assert_eq!(status.scope(), Scope::Global);
        assert_eq!(status.owner(), "PixPin");
        assert_eq!(status.bundle_id(), Some("com.pixpin.PixPin"));
    }
}
