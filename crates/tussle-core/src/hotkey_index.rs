//! The scan engine: run every source, keep what they found, and answer
//! questions about it — who owns a combo, which binding actually fires,
//! and where bindings get in each other's way.

use std::collections::HashMap;

use crate::{Binding, KeyCombo, Layer, ScanError, ScanWarning, Scope, Source};

/// Every binding every source could see, plus what went wrong on the way.
#[derive(Debug, Default)]
pub struct HotkeyIndex {
    bindings: Vec<Binding>,
    warnings: Vec<ScanWarning>,
    failures: Vec<SourceFailure>,
}

/// A source that could not run at all and therefore contributed nothing.
#[derive(Debug)]
pub struct SourceFailure {
    /// [`Source::name`] of the source that failed.
    pub source: &'static str,
    pub error: ScanError,
}

/// Who gets a combo when it is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Winner<'a> {
    /// No enabled binding claims the combo.
    Nobody,
    /// This global binding sees the key first and fires regardless of
    /// which app is frontmost.
    Global(&'a Binding),
    /// Several global bindings sit on the same first layer. Which one
    /// fires depends on registration order, which cannot be observed
    /// from outside; macOS itself flags this case in System Settings.
    Contested(Layer),
    /// Only app menu items claim the combo; whichever app is frontmost
    /// handles it.
    FrontmostApp,
}

/// Enabled bindings on one combo that get in each other's way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict<'a> {
    pub combo: KeyCombo,
    pub kind: ConflictKind,
    /// Who fires, as [`HotkeyIndex::winner`] would report.
    pub winner: Winner<'a>,
    /// Every enabled binding on the combo, ordered like
    /// [`HotkeyIndex::find`].
    pub bindings: Vec<&'a Binding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Two or more global bindings claim the combo. At most one of them
    /// can fire; the rest are dead.
    Contested,
    /// One global binding claims a combo that at least one app also uses
    /// for a menu item. That menu item never fires while the global
    /// binding is enabled.
    Shadowed,
}

impl HotkeyIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run every source in order and collect everything they report. A
    /// source that fails outright is recorded in [`failures`](Self::failures)
    /// and skipped; the others still contribute.
    pub fn scan<'a>(sources: impl IntoIterator<Item = &'a dyn Source>) -> Self {
        let mut index = Self::new();
        for source in sources {
            let started = std::time::Instant::now();
            match source.scan() {
                Ok(found) => {
                    tracing::info!(
                        source = source.name(),
                        bindings = found.bindings.len(),
                        warnings = found.warnings.len(),
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "source scan complete",
                    );
                    index.bindings.extend(found.bindings);
                    index.warnings.extend(found.warnings);
                }
                Err(error) => {
                    tracing::warn!(source = source.name(), error = %error, "source failed");
                    index.failures.push(SourceFailure {
                        source: source.name(),
                        error,
                    });
                }
            }
        }
        index
    }

    pub fn push(&mut self, binding: Binding) {
        self.bindings.push(binding);
    }

    pub fn extend(&mut self, bindings: impl IntoIterator<Item = Binding>) {
        self.bindings.extend(bindings);
    }

    /// Every binding, enabled or not, in source order.
    pub fn iter(&self) -> impl Iterator<Item = &Binding> {
        self.bindings.iter()
    }

    /// Only the bindings that currently fire.
    pub fn enabled(&self) -> impl Iterator<Item = &Binding> {
        self.bindings.iter().filter(|b| b.enabled)
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Partial-result warnings from every source, in scan order.
    pub fn warnings(&self) -> &[ScanWarning] {
        &self.warnings
    }

    /// Sources that produced nothing at all.
    pub fn failures(&self) -> &[SourceFailure] {
        &self.failures
    }

    /// Every enabled binding registered for exactly `combo`, ordered by
    /// layer (whoever sees the key first comes first), then owner, then
    /// label.
    pub fn find(&self, combo: &KeyCombo) -> Vec<&Binding> {
        let mut found: Vec<&Binding> = self.enabled().filter(|b| b.combo == *combo).collect();
        sort_by_layer(&mut found);
        found
    }

    /// Who gets `combo` when it is pressed.
    pub fn winner(&self, combo: &KeyCombo) -> Winner<'_> {
        winner_of(&self.find(combo))
    }

    /// Every combo where bindings get in each other's way, sorted by combo
    /// for stable output.
    ///
    /// Several apps reusing one shortcut in their own menus is not a
    /// conflict: each only fires in its own app. A global binding on that
    /// same combo is, because it takes the key before any app sees it.
    pub fn conflicts(&self) -> Vec<Conflict<'_>> {
        let mut out = Vec::new();
        for (combo, group) in self.by_combo() {
            let globals = group
                .iter()
                .filter(|b| b.source.scope() == Scope::Global)
                .count();
            let kind = match (globals, group.len() - globals) {
                (2.., _) => ConflictKind::Contested,
                (1, 1..) => ConflictKind::Shadowed,
                _ => continue,
            };
            out.push(Conflict {
                combo,
                kind,
                winner: winner_of(&group),
                bindings: group,
            });
        }
        out.sort_by_key(|c| c.combo.to_string());
        out
    }

    /// Enabled bindings grouped by combo, each group ordered like
    /// [`find`](Self::find). Group order is unspecified.
    fn by_combo(&self) -> HashMap<KeyCombo, Vec<&Binding>> {
        let mut groups: HashMap<KeyCombo, Vec<&Binding>> = HashMap::new();
        for b in self.enabled() {
            groups.entry(b.combo).or_default().push(b);
        }
        for group in groups.values_mut() {
            sort_by_layer(group);
        }
        groups
    }
}

/// Order bindings by who sees the keystroke first; ties broken by owner
/// and label so output is stable.
pub(crate) fn sort_by_layer(bindings: &mut [&Binding]) {
    bindings.sort_by(|a, b| {
        a.source
            .layer()
            .cmp(&b.source.layer())
            .then_with(|| a.source.owner().cmp(b.source.owner()))
            .then_with(|| a.label.cmp(&b.label))
    });
}

/// Decide the winner among bindings that share one combo and are already
/// ordered by [`sort_by_layer`].
pub(crate) fn winner_of<'a>(ordered: &[&'a Binding]) -> Winner<'a> {
    let Some(first) = ordered.first() else {
        return Winner::Nobody;
    };
    if first.source.scope() != Scope::Global {
        return Winner::FrontmostApp;
    }
    let top = first.source.layer();
    let rivals_on_top = ordered
        .iter()
        .skip(1)
        .filter(|b| b.source.layer() == top)
        .count();
    if rivals_on_top > 0 {
        Winner::Contested(top)
    } else {
        Winner::Global(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::SourceScan;
    use crate::{BindingSource, Key, Modifiers};

    fn combo(c: char) -> KeyCombo {
        KeyCombo {
            modifiers: Modifiers::CMD,
            key: Key::Char(c),
        }
    }

    fn system(id: u32, c: char, label: &str) -> Binding {
        Binding {
            combo: combo(c),
            source: BindingSource::SystemSymbolicHotkey { id: Some(id) },
            label: label.into(),
            enabled: true,
        }
    }

    fn menu(app: &str, c: char, label: &str) -> Binding {
        Binding {
            combo: combo(c),
            source: BindingSource::AppMenuItem {
                bundle_id: format!("com.example.{app}"),
                app_name: Some(app.into()),
                menu_path: vec!["File".into(), label.into()],
            },
            label: label.into(),
            enabled: true,
        }
    }

    fn status(app: &str, c: char, label: &str) -> Binding {
        Binding {
            combo: combo(c),
            source: BindingSource::StatusMenuItem {
                bundle_id: format!("com.example.{app}"),
                app_name: Some(app.into()),
                menu_path: vec![label.into()],
            },
            label: label.into(),
            enabled: true,
        }
    }

    struct Fixed(&'static str, Vec<Binding>, Vec<ScanWarning>);
    impl Source for Fixed {
        fn name(&self) -> &'static str {
            self.0
        }
        fn scan(&self) -> Result<SourceScan, ScanError> {
            Ok(SourceScan {
                bindings: self.1.clone(),
                warnings: self.2.clone(),
            })
        }
    }

    struct Broken;
    impl Source for Broken {
        fn name(&self) -> &'static str {
            "broken"
        }
        fn scan(&self) -> Result<SourceScan, ScanError> {
            Err(ScanError::Schema {
                path: "x.plist".into(),
                message: "nope".into(),
            })
        }
    }

    #[test]
    fn push_and_iter_preserve_order() {
        let mut idx = HotkeyIndex::new();
        idx.push(system(1, 'a', "a"));
        idx.push(system(2, 'b', "b"));
        let labels: Vec<_> = idx.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, vec!["a", "b"]);
    }

    #[test]
    fn extend_appends_in_order() {
        let mut idx = HotkeyIndex::new();
        idx.extend([system(1, 'x', "x"), system(2, 'y', "y")]);
        assert_eq!(idx.len(), 2);
    }

    #[test]
    fn empty_index_reports_empty() {
        let idx = HotkeyIndex::new();
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
        assert_eq!(idx.winner(&combo('z')), Winner::Nobody);
    }

    #[test]
    fn scan_collects_bindings_warnings_and_failures() {
        let warn = ScanWarning::Unresponsive {
            app: "Pages".into(),
        };
        let sources: Vec<Box<dyn Source>> = vec![
            Box::new(Fixed(
                "one",
                vec![system(64, ' ', "Spotlight")],
                vec![warn.clone()],
            )),
            Box::new(Broken),
            Box::new(Fixed("two", vec![menu("Safari", 'w', "Close")], vec![])),
        ];
        let idx = HotkeyIndex::scan(sources.iter().map(|s| s.as_ref()));
        assert_eq!(idx.len(), 2);
        assert_eq!(idx.warnings(), &[warn]);
        assert_eq!(idx.failures().len(), 1);
        assert_eq!(idx.failures()[0].source, "broken");
    }

    #[test]
    fn find_orders_system_before_app_menus_and_skips_disabled() {
        let mut idx = HotkeyIndex::new();
        idx.push(menu("Zed", 'k', "Command Palette"));
        idx.push(menu("Alpha", 'k', "Kick"));
        idx.push(system(9, 'k', "Some system thing"));
        let mut off = menu("Off", 'k', "Disabled");
        off.enabled = false;
        idx.push(off);

        let owners: Vec<_> = idx
            .find(&combo('k'))
            .iter()
            .map(|b| b.source.owner().to_string())
            .collect();
        assert_eq!(owners, vec!["macOS", "Alpha", "Zed"]);
    }

    #[test]
    fn global_binding_wins_over_app_menus() {
        let mut idx = HotkeyIndex::new();
        idx.push(menu("Safari", ' ', "Search"));
        idx.push(system(64, ' ', "Spotlight"));
        match idx.winner(&combo(' ')) {
            Winner::Global(b) => assert_eq!(b.label, "Spotlight"),
            other => panic!("expected the system binding to win, got {other:?}"),
        }
    }

    #[test]
    fn only_app_menus_means_frontmost_app_decides() {
        let mut idx = HotkeyIndex::new();
        idx.push(menu("Safari", 'w', "Close Window"));
        idx.push(menu("Mail", 'w', "Close"));
        assert_eq!(idx.winner(&combo('w')), Winner::FrontmostApp);
    }

    #[test]
    fn two_globals_on_the_same_layer_are_contested() {
        let mut idx = HotkeyIndex::new();
        idx.push(system(64, 'q', "Show Spotlight search"));
        idx.push(system(65, 'q', "Show Finder search window"));
        idx.push(menu("Safari", 'q', "Quit"));
        assert_eq!(idx.winner(&combo('q')), Winner::Contested(Layer::System));
    }

    #[test]
    fn apps_reusing_a_shortcut_are_not_a_conflict() {
        let mut idx = HotkeyIndex::new();
        idx.push(menu("Safari", 'w', "Close Window"));
        idx.push(menu("Mail", 'w', "Close"));
        idx.push(menu("Terminal", 'w', "Close Window"));
        assert!(idx.conflicts().is_empty());
    }

    #[test]
    fn global_over_app_menu_is_shadowed() {
        let mut idx = HotkeyIndex::new();
        idx.push(menu("Safari", ' ', "Search"));
        idx.push(system(64, ' ', "Spotlight"));
        let conflicts = idx.conflicts();
        assert_eq!(conflicts.len(), 1);
        let c = &conflicts[0];
        assert_eq!(c.kind, ConflictKind::Shadowed);
        assert_eq!(c.combo, combo(' '));
        assert!(matches!(c.winner, Winner::Global(b) if b.label == "Spotlight"));
        assert_eq!(c.bindings.len(), 2);
        assert_eq!(c.bindings[0].label, "Spotlight");
    }

    #[test]
    fn two_globals_are_contested_and_menus_ride_along() {
        let mut idx = HotkeyIndex::new();
        idx.push(menu("Warp", '1', "Left Panel"));
        idx.push(system(118, '1', "Switch to Desktop 1"));
        idx.push(system(119, '1', "Switch to Desktop 2"));
        let conflicts = idx.conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, ConflictKind::Contested);
        assert_eq!(conflicts[0].winner, Winner::Contested(Layer::System));
        assert_eq!(conflicts[0].bindings.len(), 3);
    }

    #[test]
    fn conflicts_are_sorted_by_combo_and_ignore_disabled() {
        let mut idx = HotkeyIndex::new();
        idx.push(system(1, 'z', "Z"));
        idx.push(menu("App", 'z', "z item"));
        idx.push(system(2, 'a', "A"));
        idx.push(menu("App", 'a', "a item"));
        let mut off = system(3, 'm', "off");
        off.enabled = false;
        idx.push(off);
        idx.push(menu("App", 'm', "m item"));
        let combos: Vec<String> = idx
            .conflicts()
            .iter()
            .map(|c| c.combo.to_string())
            .collect();
        assert_eq!(combos, vec!["cmd+a", "cmd+z"]);
    }

    #[test]
    fn status_bar_shortcut_beats_app_menus_but_not_the_system() {
        let mut idx = HotkeyIndex::new();
        idx.push(menu("Warp", '1', "Left Panel"));
        idx.push(status("PixPin", '1', "截图"));
        assert!(
            matches!(idx.winner(&combo('1')), Winner::Global(b) if b.source.owner() == "PixPin")
        );
        let conflicts = idx.conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, ConflictKind::Shadowed);

        idx.push(system(118, '1', "Switch to Desktop 1"));
        assert!(
            matches!(idx.winner(&combo('1')), Winner::Global(b) if b.source.owner() == "macOS")
        );
        assert_eq!(idx.conflicts()[0].kind, ConflictKind::Contested);
    }

    #[test]
    fn two_status_bar_apps_on_one_combo_are_contested() {
        let mut idx = HotkeyIndex::new();
        idx.push(status("PixPin", 'x', "截图"));
        idx.push(status("CleanShot", 'x', "Capture"));
        assert_eq!(
            idx.winner(&combo('x')),
            Winner::Contested(Layer::GlobalHotkey)
        );
    }
}
