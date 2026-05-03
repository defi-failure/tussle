//! Integration tests for `tussle_core::sources::nsuserkeyequivalents`.

use std::path::PathBuf;

use tussle_core::sources::nsuserkeyequivalents;
use tussle_core::{BindingSource, Key, KeyCombo, Modifiers};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("nsuserkeyequivalents")
        .join(name)
}

#[test]
#[ignore = "parse not yet implemented"]
fn parses_synthetic_app_overrides() {
    let bindings =
        nsuserkeyequivalents::parse(&fixture("com.example.app.plist")).expect("parse should succeed");

    // The fixture has four `NSUserKeyEquivalents` entries plus an unrelated
    // setting at the top level which must be ignored.
    assert_eq!(bindings.len(), 4);

    // "New" → @~n → ⌘⌥N
    let new = bindings
        .iter()
        .find(|b| matches!(
            &b.source,
            BindingSource::AppMenuOverride { menu_item, .. } if menu_item == "New"
        ))
        .expect("'New' override should be parsed");
    assert_eq!(
        new.combo,
        KeyCombo {
            modifiers: Modifiers::CMD | Modifiers::OPT,
            key: Key::Char('n'),
        }
    );
    if let BindingSource::AppMenuOverride { bundle_id, .. } = &new.source {
        assert_eq!(bundle_id, "com.example.app");
    } else {
        unreachable!()
    }

    // "Save All" → @$s → ⌘⇧S
    let save_all = bindings
        .iter()
        .find(|b| matches!(
            &b.source,
            BindingSource::AppMenuOverride { menu_item, .. } if menu_item == "Save All"
        ))
        .expect("'Save All' override should be parsed");
    assert_eq!(
        save_all.combo,
        KeyCombo {
            modifiers: Modifiers::CMD | Modifiers::SHIFT,
            key: Key::Char('s'),
        }
    );

    // "Open Recent" → @^o → ⌘⌃O
    let open_recent = bindings
        .iter()
        .find(|b| matches!(
            &b.source,
            BindingSource::AppMenuOverride { menu_item, .. } if menu_item == "Open Recent"
        ))
        .expect("'Open Recent' override should be parsed");
    assert_eq!(
        open_recent.combo,
        KeyCombo {
            modifiers: Modifiers::CMD | Modifiers::CTRL,
            key: Key::Char('o'),
        }
    );

    // "Reload" → @r → ⌘R (no extra modifiers)
    let reload = bindings
        .iter()
        .find(|b| matches!(
            &b.source,
            BindingSource::AppMenuOverride { menu_item, .. } if menu_item == "Reload"
        ))
        .expect("'Reload' override should be parsed");
    assert_eq!(
        reload.combo,
        KeyCombo {
            modifiers: Modifiers::CMD,
            key: Key::Char('r'),
        }
    );
}
