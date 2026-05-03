//! Integration tests for `tussle_core::sources::symbolichotkeys`.
//!
//! Drive the parser against real `com.apple.symbolichotkeys.plist` samples
//! checked in under `tests/fixtures/symbolichotkeys/`.

use std::path::PathBuf;

use tussle_core::sources::symbolichotkeys;
use tussle_core::{BindingSource, Key, KeyCombo, Modifiers, NamedKey};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("symbolichotkeys")
        .join(name)
}

#[test]
fn parses_customized_fixture() {
    let bindings =
        symbolichotkeys::scan(&fixture("customized.plist")).expect("parse should succeed");

    // ID 64 (Show Spotlight search) is bound to ⌘Space.
    let spotlight = bindings
        .iter()
        .find(|b| matches!(&b.source, BindingSource::SystemSymbolicHotkey { id: 64 }))
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
        .find(|b| matches!(&b.source, BindingSource::SystemSymbolicHotkey { id: 65 }))
        .expect("Finder search binding should be parsed");
    assert_eq!(
        finder_search.combo,
        KeyCombo {
            modifiers: Modifiers::CMD | Modifiers::OPT,
            key: Key::Named(NamedKey::Space),
        }
    );
    assert_eq!(finder_search.label, "Show Finder search window");

    // Disabled entries (e.g. ID 17, ID 22) must NOT appear in the output.
    for disabled_id in [17u32, 18, 19, 20, 21, 22, 23, 24, 25, 26] {
        assert!(
            !bindings.iter().any(|b| matches!(
                &b.source,
                BindingSource::SystemSymbolicHotkey { id } if *id == disabled_id
            )),
            "disabled binding id {disabled_id} should be filtered out",
        );
    }

    // ID 32 (Mission Control) is not in this fixture's plist but should be
    // surfaced from the macOS defaults table at ⌃↑.
    let mission_control = bindings
        .iter()
        .find(|b| matches!(&b.source, BindingSource::SystemSymbolicHotkey { id: 32 }))
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
        .find(|b| matches!(&b.source, BindingSource::SystemSymbolicHotkey { id: 118 }))
        .expect("Switch to Desktop 1 default should be merged in");
    assert_eq!(
        desktop_1.combo,
        KeyCombo {
            modifiers: Modifiers::CTRL,
            key: Key::Char('1'),
        }
    );

    // ID 79 (Move left a space) is enabled-no-value in the fixture and
    // should fall back to the macOS default ⌃←.
    let space_left = bindings
        .iter()
        .find(|b| matches!(&b.source, BindingSource::SystemSymbolicHotkey { id: 79 }))
        .expect("enabled-with-default ID 79 should pick up the macOS default");
    assert_eq!(
        space_left.combo,
        KeyCombo {
            modifiers: Modifiers::CTRL,
            key: Key::Named(NamedKey::Left),
        }
    );
}
