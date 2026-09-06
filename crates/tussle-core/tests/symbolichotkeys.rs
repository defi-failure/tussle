//! Integration tests for `tussle_core::sources::symbolichotkeys`.
//!
//! Drive the parser against real `com.apple.symbolichotkeys.plist` samples
//! checked in under `tests/fixtures/symbolichotkeys/`.

use std::path::PathBuf;

use tussle_core::sources::symbolichotkeys::SymbolicHotkeys;
use tussle_core::{BindingSource, Key, KeyCombo, Modifiers, NamedKey, Source};

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
                BindingSource::SystemSymbolicHotkey { id: Some(64) }
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
                BindingSource::SystemSymbolicHotkey { id: Some(65) }
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
            matches!(&b.source, BindingSource::SystemSymbolicHotkey { id: Some(id) } if *id == disabled_id)
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
                BindingSource::SystemSymbolicHotkey { id: Some(32) }
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
                BindingSource::SystemSymbolicHotkey { id: Some(118) }
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
                BindingSource::SystemSymbolicHotkey { id: Some(79) }
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
            |b| matches!(&b.source, BindingSource::SystemSymbolicHotkey { id: Some(i) } if *i == id),
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
