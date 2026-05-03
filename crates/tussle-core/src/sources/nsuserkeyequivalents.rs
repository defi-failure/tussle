//! Parser for per-app `NSUserKeyEquivalents` overrides.
//!
//! Each app's `~/Library/Preferences/<bundle_id>.plist` may contain an
//! `NSUserKeyEquivalents` dictionary that the user populates via
//! System Settings → Keyboard → Keyboard Shortcuts → App Shortcuts.
//! Keys are menu item titles (`"New"`, `"Save All"`); values are NSText
//! keystroke shorthand:
//!
//!   - `@` = Command
//!   - `~` = Option
//!   - `$` = Shift
//!   - `^` = Control
//!
//! followed by the literal key character. So `@~n` denotes ⌘⌥N.
//!
//! See Apple's [NSEvent keyEquivalent docs][docs].
//!
//! [docs]: https://developer.apple.com/documentation/appkit/nsevent/keyequivalent

use std::path::Path;

use crate::{Binding, ScanError};

/// Parse a single `<bundle_id>.plist` for its `NSUserKeyEquivalents` dict.
///
/// Returns an empty `Vec` (not an error) when the file has no overrides —
/// most apps don't, so an empty result is the common case.
pub fn parse(path: &Path) -> Result<Vec<Binding>, ScanError> {
    let _ = path;
    todo!("nsuserkeyequivalents parser not yet implemented")
}

/// Walk every plist under `prefs_dir` and aggregate menu-item overrides.
pub fn scan(prefs_dir: &Path) -> Result<Vec<Binding>, ScanError> {
    let _ = prefs_dir;
    todo!("nsuserkeyequivalents directory walker not yet implemented")
}
