//! Engine for discovering and reasoning about macOS keyboard shortcuts
//! across system, application menus, and third-party launchers.

mod binding;
pub mod capture;
mod combo;
mod error;
mod hotkey_index;
pub mod sources;

pub use binding::{Binding, BindingSource};
pub use combo::{Key, KeyCombo, Modifiers, NamedKey};
pub use error::ScanError;
pub use hotkey_index::HotkeyIndex;
pub use sources::Source;
