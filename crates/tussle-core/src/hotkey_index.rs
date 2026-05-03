use crate::Binding;

/// Container for all discovered hotkey bindings across sources.
///
/// At M1 this is an append-only list. Reverse lookup (`find_by_combo`) and
/// conflict detection are added when later milestones introduce more sources.
#[derive(Debug, Default, Clone)]
pub struct HotkeyIndex {
    bindings: Vec<Binding>,
}

impl HotkeyIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, binding: Binding) {
        self.bindings.push(binding);
    }

    pub fn extend(&mut self, bindings: impl IntoIterator<Item = Binding>) {
        self.bindings.extend(bindings);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Binding> {
        self.bindings.iter()
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BindingSource, Key, KeyCombo, Modifiers};

    fn fake_binding(label: &str) -> Binding {
        Binding {
            combo: KeyCombo {
                modifiers: Modifiers::CMD,
                key: Key::Char('a'),
            },
            source: BindingSource::SystemSymbolicHotkey { id: 0 },
            label: label.into(),
        }
    }

    #[test]
    fn push_and_iter_preserve_order() {
        let mut idx = HotkeyIndex::new();
        idx.push(fake_binding("a"));
        idx.push(fake_binding("b"));
        let labels: Vec<_> = idx.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, vec!["a", "b"]);
    }

    #[test]
    fn extend_appends_in_order() {
        let mut idx = HotkeyIndex::new();
        idx.extend([fake_binding("x"), fake_binding("y")]);
        assert_eq!(idx.len(), 2);
    }

    #[test]
    fn empty_index_reports_empty() {
        let idx = HotkeyIndex::new();
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
    }
}
