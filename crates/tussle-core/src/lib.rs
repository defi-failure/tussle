//! Engine for discovering and reasoning about macOS keyboard shortcuts
//! across system, application menus, and third-party launchers.

mod binding;
pub mod capture;
mod combo;
mod error;
mod hotkey_index;
pub mod sources;

pub use binding::{Binding, BindingSource, Layer, Scope, SystemDispatch};
pub use combo::{ComboToken, Key, KeyCombo, Modifiers, NamedKey};
pub use error::{ScanError, ScanWarning};
pub use hotkey_index::{Conflict, ConflictKind, HotkeyIndex, SourceFailure, Winner};
pub use sources::{Source, SourceScan};
