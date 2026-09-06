//! Integration tests for `tussle_core::sources::symbolichotkeys`.
//!
//! Drive the parser against real `com.apple.symbolichotkeys.plist` samples
//! checked in under `tests/fixtures/symbolichotkeys/`.

use std::path::PathBuf;

use tussle_core::sources::symbolichotkeys::{LiveHotkey, LiveTable, SymbolicHotkeys};
use tussle_core::{BindingSource, Key, KeyCombo, Modifiers, NamedKey, Source, SystemDispatch};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("symbolichotkeys")
        .join(name)
}

#[test]
fn parses_customized_fixture() {
    let bindings = SymbolicHotkeys::new(fixture("customized.plist"))
        .scan()
        .expect("parse should succeed")
        .bindings;

    // ID 64 (Show Spotlight search) is bound to ⌘Space.
    let spotlight = bindings
        .iter()
        .find(|b| {
            matches!(
                &b.source,
                BindingSource::SystemSymbolicHotkey { id: Some(64), .. }
            )
        })
        .expect("Spotlight binding should be parsed");
    assert_eq!(
        spotlight.combo,
        KeyCombo {
            modifiers: Modifiers::CMD,
            key: Key::Named(NamedKey::Space),
        }
    );
    assert_eq!(spotlight.label, "Show Spotlight search");

    // ID 65 (Show Finder search window) is bound to ⌥⌘Space.
    let finder_search = bindings
        .iter()
        .find(|b| {
            matches!(
                &b.source,
                BindingSource::SystemSymbolicHotkey { id: Some(65), .. }
            )
        })
        .expect("Finder search binding should be parsed");
    assert_eq!(
        finder_search.combo,
        KeyCombo {
            modifiers: Modifiers::CMD | Modifiers::OPT,
            key: Key::Named(NamedKey::Space),
        }
    );
    assert_eq!(finder_search.label, "Show Finder search window");

    // These have neither a stored combo nor a known default, so they are
    // not reported at all; if a future default table adds one, it must
    // still come out disabled.
    for disabled_id in [17u32, 18, 19, 20, 21, 22, 23, 24, 25, 26] {
        for b in bindings.iter().filter(|b| {
            matches!(&b.source, BindingSource::SystemSymbolicHotkey { id: Some(id), .. } if *id == disabled_id)
        }) {
            assert!(!b.enabled, "disabled binding id {disabled_id} must not be enabled");
        }
    }

    // ID 32 (Mission Control) is not in this fixture's plist but should be
    // surfaced from the macOS defaults table at ⌃↑.
    let mission_control = bindings
        .iter()
        .find(|b| {
            matches!(
                &b.source,
                BindingSource::SystemSymbolicHotkey { id: Some(32), .. }
            )
        })
        .expect("Mission Control default should be merged in");
    assert_eq!(
        mission_control.combo,
        KeyCombo {
            modifiers: Modifiers::CTRL,
            key: Key::Named(NamedKey::Up),
        }
    );

    // ID 118 (Switch to Desktop 1) is also not in the fixture but should
    // appear via defaults at ⌃1.
    let desktop_1 = bindings
        .iter()
        .find(|b| {
            matches!(
                &b.source,
                BindingSource::SystemSymbolicHotkey { id: Some(118), .. }
            )
        })
        .expect("Switch to Desktop 1 default should be merged in");
    assert_eq!(
        desktop_1.combo,
        KeyCombo {
            modifiers: Modifiers::CTRL,
            key: Key::Char('1'),
        }
    );
    // No plist entry for 118 means macOS's own default applies, and the
    // desktop-switching shortcuts ship switched off.
    assert!(
        !desktop_1.enabled,
        "Switch to Desktop 1 is off unless the plist enables it"
    );

    // ID 79 (Move left a space) is enabled-no-value in the fixture and
    // should fall back to the macOS default ⌃←.
    let space_left = bindings
        .iter()
        .find(|b| {
            matches!(
                &b.source,
                BindingSource::SystemSymbolicHotkey { id: Some(79), .. }
            )
        })
        .expect("enabled-with-default ID 79 should pick up the macOS default");
    assert_eq!(
        space_left.combo,
        KeyCombo {
            modifiers: Modifiers::CTRL,
            key: Key::Named(NamedKey::Left),
        }
    );
}

#[test]
fn keeps_disabled_hotkeys_with_known_combos() {
    let bindings = SymbolicHotkeys::new(fixture("disabled-with-known-combos.plist"))
        .scan()
        .expect("parse should succeed")
        .bindings;

    let by_id = |id: u32| {
        bindings.iter().find(
            |b| matches!(&b.source, BindingSource::SystemSymbolicHotkey { id: Some(i), .. } if *i == id),
        )
    };

    // ID 64 is disabled with no stored combo: reported with the macOS
    // default (⌘Space) and enabled = false.
    let spotlight = by_id(64).expect("disabled Spotlight should still be reported");
    assert!(!spotlight.enabled);
    assert_eq!(
        spotlight.combo,
        KeyCombo {
            modifiers: Modifiers::CMD,
            key: Key::Named(NamedKey::Space),
        }
    );

    // ID 65 is disabled but the plist still stores a custom ⌃⌥Space.
    let finder = by_id(65).expect("disabled Finder search should still be reported");
    assert!(!finder.enabled);
    assert_eq!(
        finder.combo,
        KeyCombo {
            modifiers: Modifiers::CTRL | Modifiers::OPT,
            key: Key::Named(NamedKey::Space),
        }
    );
}

/// The live table captured on a Tahoe machine, see the fixture header.
fn live_table() -> Vec<LiveHotkey> {
    let text = std::fs::read_to_string(fixture("live-table-tahoe.tsv")).expect("fixture");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut cols = l.split('\t');
            let vk: u16 = cols.next().unwrap().parse().unwrap();
            let mods = cols.next().unwrap().trim_start_matches("0x");
            let carbon_modifiers = u32::from_str_radix(mods, 16).unwrap();
            let enabled = cols.next().unwrap() == "true";
            LiveHotkey {
                vk,
                carbon_modifiers,
                enabled,
            }
        })
        .collect()
}

#[test]
fn live_table_is_the_truth_and_the_plist_supplies_labels() {
    let rows = live_table();
    assert_eq!(rows.len(), 230);
    let scan = SymbolicHotkeys::new(fixture("customized.plist"))
        .with_live_table(LiveTable::Snapshot(rows))
        .scan()
        .expect("scan should succeed");
    let bindings = scan.bindings;
    let by_combo = |text: &str| {
        let combo = KeyCombo::parse(text).unwrap();
        bindings
            .iter()
            .filter(|b| b.combo == combo)
            .collect::<Vec<_>>()
    };

    // ⌘Tab has no plist id but is a live, enabled system shortcut.
    let tab = by_combo("cmd+tab");
    assert_eq!(tab.len(), 1);
    assert!(tab[0].enabled);
    assert_eq!(
        tab[0].source,
        BindingSource::SystemSymbolicHotkey {
            id: None,
            dispatch: SystemDispatch::BeforeApps,
        }
    );
    assert_eq!(tab[0].label, "Switch to next app");

    // ⌃1 (Switch to Desktop 1) is in the table but switched off, and the
    // defaults table ties the combo to id 118.
    let d1 = by_combo("ctrl+1");
    assert_eq!(d1.len(), 1);
    assert!(!d1[0].enabled);
    assert_eq!(
        d1[0].source,
        BindingSource::SystemSymbolicHotkey {
            id: Some(118),
            dispatch: SystemDispatch::BeforeApps,
        }
    );
    assert_eq!(d1[0].label, "Switch to Desktop 1");

    // ⌘Space is Spotlight, labelled through id 64.
    let spot = by_combo("cmd+space");
    assert_eq!(spot.len(), 1);
    assert!(spot[0].enabled);
    assert_eq!(spot[0].label, "Show Spotlight search");

    // fn on F-keys is folded away: ⌃F2 comes out as ctrl+f2.
    assert_eq!(by_combo("ctrl+f2").len(), 1);

    // ⌘M is in the table too, but as the standard Minimize menu item: it
    // must not be able to shadow an app's own ⌘M.
    let minimize = by_combo("cmd+m");
    assert_eq!(minimize.len(), 1);
    assert_eq!(minimize[0].label, "Minimize");
    assert_eq!(
        minimize[0].source,
        BindingSource::SystemSymbolicHotkey {
            id: None,
            dispatch: SystemDispatch::StandardMenuItem,
        }
    );
    assert_eq!(minimize[0].source.layer(), tussle_core::Layer::AppMenu);

    // Entries with no key are skipped and duplicates collapse: the count of
    // enabled bindings equals the distinct enabled combos in the table.
    let expected_enabled: std::collections::HashSet<KeyCombo> = live_table()
        .iter()
        .filter(|r| r.enabled)
        .filter_map(|r| r.combo())
        .collect();
    let enabled = bindings.iter().filter(|b| b.enabled).count();
    assert_eq!(enabled, expected_enabled.len());
    assert!(bindings.len() < 230);
}
