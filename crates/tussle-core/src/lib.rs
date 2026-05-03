//! Engine for discovering and reasoning about macOS keyboard shortcuts
//! across system, application menus, and third-party launchers.

mod binding;
mod hotkey_index;
mod key;
mod key_combo;
mod modifiers;

pub use binding::{Binding, BindingSource};
pub use hotkey_index::HotkeyIndex;
pub use key::{Key, NamedKey};
pub use key_combo::KeyCombo;
pub use modifiers::Modifiers;
