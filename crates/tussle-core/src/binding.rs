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
}

/// The system, app, or launcher that owns a binding.
///
/// New variants are added as parsers come online; keep this enum extensible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingSource {
    /// A macOS system shortcut sourced from
    /// `~/Library/Preferences/com.apple.symbolichotkeys.plist`.
    ///
    /// `id` is the symbolic hotkey numeric identifier (e.g. 64 = Spotlight).
    SystemSymbolicHotkey { id: u32 },

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
}

impl BindingSource {
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
            } => app_name.as_deref().unwrap_or(bundle_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_for_symbolic_hotkey_is_macos() {
        let s = BindingSource::SystemSymbolicHotkey { id: 64 };
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
}
