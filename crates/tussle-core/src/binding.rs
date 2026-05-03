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
}

impl BindingSource {
    /// Short human-readable identifier for whoever owns this binding —
    /// answers "who took this keystroke?" not "what does it do?".
    pub fn owner(&self) -> &'static str {
        match self {
            BindingSource::SystemSymbolicHotkey { .. } => "macOS",
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
}
