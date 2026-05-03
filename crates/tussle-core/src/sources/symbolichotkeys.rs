//! Parser for `~/Library/Preferences/com.apple.symbolichotkeys.plist`.
//!
//! The plist holds user customizations of macOS system shortcuts (Spotlight,
//! Mission Control, screenshots, ...). Each entry is a numeric ID mapping to
//! `{ enabled, value: { parameters: [char_code, virtual_keycode, mask], type } }`.
//! Defaults that the user has not overridden are NOT in this file — they are
//! hard-coded in macOS itself.

use std::path::Path;

use crate::{Binding, ScanError};

/// Parse a symbolichotkeys plist into the bindings it represents.
///
/// Disabled entries are filtered out. Entries lacking a `value` dict (which
/// means "use macOS default") are skipped at this layer because we cannot
/// know the default from this file alone.
pub fn scan(path: &Path) -> Result<Vec<Binding>, ScanError> {
    let _ = path;
    todo!("symbolichotkeys parser not yet implemented")
}
