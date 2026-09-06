//! Parser for `~/Library/Preferences/com.apple.symbolichotkeys.plist`.
//!
//! macOS stores user customizations of system shortcuts (Spotlight, Mission
//! Control, screenshots, ...) as numeric IDs in this plist. Each entry is
//! `{ enabled, value: { parameters: [char_code, virtual_keycode, mask], type } }`.
//!
//! macOS DEFAULTS are NOT stored in this file — they live hardcoded in the
//! system. We therefore maintain `macos_defaults()` below and merge it with
//! the plist contents to produce a complete picture: defaults overlaid by
//! whatever the user has customized or disabled.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::combo::vk_to_named;

use crate::{Binding, BindingSource, Key, KeyCombo, Modifiers, ScanError, ScanWarning};
use known::{builtin_for, dispatch_for, known_hotkeys, label_for};

use super::{Source, SourceScan};

mod known;
mod live;

pub use live::{LiveHotkey, NO_KEY};

/// Label of a live-table entry nothing could name. The index may replace
/// it with what apps call the same combo, see
/// [`HotkeyIndex`](crate::HotkeyIndex).
pub const UNLABELLED_SYSTEM_SHORTCUT: &str = "macOS shortcut";

/// Reads `com.apple.symbolichotkeys.plist` and merges its contents with
/// macOS's hardcoded default table.
#[derive(Debug, Clone)]
pub struct SymbolicHotkeys {
    plist_path: PathBuf,
    live: LiveTable,
}

/// Where the effective system table comes from.
#[derive(Debug, Clone)]
pub enum LiveTable {
    /// Only the plist plus the built-in defaults table. Deterministic;
    /// what tests use, and the fallback when the system table cannot be
    /// read.
    None,
    /// Ask macOS via `CopySymbolicHotKeys()`.
    System,
    /// A captured table, for tests and for reproducing another machine.
    Snapshot(Vec<LiveHotkey>),
}

impl SymbolicHotkeys {
    /// Read `plist_path` and merge it with the built-in defaults table.
    pub fn new(plist_path: PathBuf) -> Self {
        Self {
            plist_path,
            live: LiveTable::None,
        }
    }

    /// Use the given table as the truth for combos and enabled state; the
    /// plist and defaults then only supply ids and labels.
    pub fn with_live_table(mut self, live: LiveTable) -> Self {
        self.live = live;
        self
    }
}

impl Source for SymbolicHotkeys {
    fn name(&self) -> &'static str {
        "symbolichotkeys"
    }

    fn scan(&self) -> Result<SourceScan, ScanError> {
        let rows = match &self.live {
            LiveTable::None => None,
            LiveTable::Snapshot(rows) => Some(rows.clone()),
            LiveTable::System => match live::read_system_table() {
                Ok(rows) => Some(rows),
                Err(message) => {
                    tracing::warn!(%message, "system hotkey table unavailable, using plist only");
                    None
                }
            },
        };
        let Some(rows) = rows else {
            return scan(&self.plist_path).map(SourceScan::from);
        };
        // With the live table in hand the plist is only a source of ids and
        // labels, so a missing or broken one degrades to unlabeled entries.
        let mut out = SourceScan::default();
        let overrides = match parse_overrides(&self.plist_path) {
            Ok(overrides) => overrides,
            Err(e) => {
                out.warnings.push(ScanWarning::Skipped {
                    path: self.plist_path.clone(),
                    message: e.to_string(),
                });
                HashMap::new()
            }
        };
        out.bindings = merge_live(&rows, &overrides);
        Ok(out)
    }
}

/// `parameters` array index for the printable character code, the macOS
/// virtual keycode, and the NSEvent modifier mask, respectively.
const PARAM_CHAR: usize = 0;
const PARAM_VK: usize = 1;
const PARAM_MASK: usize = 2;

/// Sentinel value Apple writes when a parameter slot is unset.
const UNSET: i64 = 65535;

// NSEvent modifier flag bits, from `AppKit/NSEvent.h`:
//   NSEventModifierFlagShift    = 1 << 17  = 0x0002_0000
//   NSEventModifierFlagControl  = 1 << 18  = 0x0004_0000
//   NSEventModifierFlagOption   = 1 << 19  = 0x0008_0000
//   NSEventModifierFlagCommand  = 1 << 20  = 0x0010_0000
//   NSEventModifierFlagFunction = 1 << 23  = 0x0080_0000
const NS_SHIFT: u64 = 1 << 17;
const NS_CTRL: u64 = 1 << 18;
const NS_OPT: u64 = 1 << 19;
const NS_CMD: u64 = 1 << 20;
const NS_FN: u64 = 1 << 23;

/// What the user's plist says about a particular hotkey ID.
#[derive(Debug, Clone, Copy)]
enum Override {
    /// User explicitly disabled this shortcut. Carries the custom combo the
    /// plist still stores for it, if any, so the entry can be reported as
    /// present-but-off.
    Disabled(Option<KeyCombo>),
    /// User has the shortcut enabled but has not customized the combo —
    /// macOS uses its built-in default.
    EnabledWithDefault,
    /// User has bound this shortcut to a specific combo.
    Custom(KeyCombo),
}

/// Parse the plist and merge with macOS's default symbolic hotkey table to
/// produce the final set of bindings. Disabled entries are kept with
/// `enabled = false` whenever a combo is known for them.
fn scan(path: &Path) -> Result<Vec<Binding>, ScanError> {
    let overrides = parse_overrides(path)?;
    let defaults: Vec<_> = known_hotkeys()
        .into_iter()
        .filter(|k| k.combo.is_some())
        .collect();

    let mut bindings = Vec::new();
    let mut handled: HashSet<u32> = HashSet::new();

    // Pass 1: every ID we know a default for. Apply overrides on top. An
    // ID with no plist entry at all is in whatever state macOS ships it,
    // which is not always "on".
    for d in &defaults {
        handled.insert(d.id);
        let default_combo = d.combo.expect("filtered to entries with a combo");
        let (combo, enabled) = match overrides.get(&d.id) {
            Some(Override::Disabled(custom)) => (custom.unwrap_or(default_combo), false),
            Some(Override::Custom(c)) => (*c, true),
            Some(Override::EnabledWithDefault) => (default_combo, true),
            None => (default_combo, d.enabled),
        };
        bindings.push(emit(d.id, combo, enabled));
    }

    // Pass 2: IDs the user has customized for which we have no default. Surface
    // them with a generic label so unmapped customizations remain visible.
    for (id, entry) in &overrides {
        if handled.contains(id) {
            continue;
        }
        match entry {
            Override::Custom(c) => bindings.push(emit(*id, *c, true)),
            Override::Disabled(Some(c)) => bindings.push(emit(*id, *c, false)),
            Override::Disabled(None) | Override::EnabledWithDefault => {}
        }
    }

    bindings.sort_by_key(|b| {
        if let BindingSource::SystemSymbolicHotkey { id: Some(id), .. } = &b.source {
            *id
        } else {
            unreachable!("this parser only emits SystemSymbolicHotkey bindings")
        }
    });

    Ok(bindings)
}

fn emit(id: u32, combo: KeyCombo, enabled: bool) -> Binding {
    Binding {
        combo,
        source: BindingSource::SystemSymbolicHotkey {
            id: Some(id),
            dispatch: dispatch_for(Some(id), &combo),
        },
        label: label_for(id)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Symbolic hotkey #{id}")),
        enabled,
    }
}

/// Bindings from the live table, labelled through the plist and the
/// defaults table wherever a combo can be tied to an id.
fn merge_live(rows: &[LiveHotkey], overrides: &HashMap<u32, Override>) -> Vec<Binding> {
    // Defaults first, then the user's own combos on top: if Spotlight was
    // moved to ⌥Space, ⌥Space is 64 and ⌘Space no longer is.
    let mut ids_by_combo: HashMap<KeyCombo, u32> = HashMap::new();
    for d in known_hotkeys() {
        if let Some(combo) = d.combo {
            ids_by_combo.insert(combo, d.id);
        }
    }
    for (id, entry) in overrides {
        match entry {
            Override::Custom(c) | Override::Disabled(Some(c)) => {
                ids_by_combo.insert(*c, *id);
            }
            Override::Disabled(None) | Override::EnabledWithDefault => {}
        }
    }

    let mut seen: HashSet<(KeyCombo, bool)> = HashSet::new();
    let mut bindings = Vec::new();
    for row in rows {
        let Some(combo) = row.combo() else {
            continue;
        };
        if !seen.insert((combo, row.enabled)) {
            continue;
        }
        let id = ids_by_combo.get(&combo).copied();
        let label = id
            .and_then(label_for)
            .map(str::to_owned)
            .or_else(|| builtin_for(&combo).map(|b| b.label.to_owned()))
            .or_else(|| id.map(|id| format!("Symbolic hotkey #{id}")))
            .unwrap_or_else(|| UNLABELLED_SYSTEM_SHORTCUT.to_owned());
        bindings.push(Binding {
            combo,
            source: BindingSource::SystemSymbolicHotkey {
                id,
                dispatch: dispatch_for(id, &combo),
            },
            label,
            enabled: row.enabled,
        });
    }
    bindings
}

fn parse_overrides(path: &Path) -> Result<HashMap<u32, Override>, ScanError> {
    let bytes = std::fs::read(path).map_err(|source| ScanError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let value: plist::Value = plist::from_bytes(&bytes).map_err(|e| ScanError::Schema {
        path: path.to_path_buf(),
        message: format!("plist parse: {e}"),
    })?;

    let root = value.as_dictionary().ok_or_else(|| ScanError::Schema {
        path: path.to_path_buf(),
        message: "root is not a dictionary".into(),
    })?;

    let entries = root
        .get("AppleSymbolicHotKeys")
        .and_then(|v| v.as_dictionary())
        .ok_or_else(|| ScanError::Schema {
            path: path.to_path_buf(),
            message: "missing AppleSymbolicHotKeys dict".into(),
        })?;

    let mut map = HashMap::new();
    for (id_str, entry) in entries {
        let Ok(id) = id_str.parse::<u32>() else {
            continue;
        };
        let Some(entry_dict) = entry.as_dictionary() else {
            continue;
        };

        let enabled = entry_dict
            .get("enabled")
            .and_then(|v| v.as_boolean())
            .unwrap_or(true);
        let custom = custom_combo(entry_dict);
        let entry = match (enabled, custom) {
            (false, custom) => Override::Disabled(custom),
            (true, Some(combo)) => Override::Custom(combo),
            (true, None) => Override::EnabledWithDefault,
        };
        map.insert(id, entry);
    }

    Ok(map)
}

/// The combo stored under `value.parameters`, or `None` when the entry has
/// no value, too few parameters, or Apple's `(65535, 65535, *)` "no
/// override" placeholder.
fn custom_combo(entry: &plist::Dictionary) -> Option<KeyCombo> {
    let params = entry
        .get("value")
        .and_then(|v| v.as_dictionary())?
        .get("parameters")
        .and_then(|v| v.as_array())?;
    if params.len() < 3 {
        return None;
    }
    let char_code = params[PARAM_CHAR].as_signed_integer().unwrap_or(UNSET);
    let vk = params[PARAM_VK].as_signed_integer().unwrap_or(UNSET);
    let mask = params[PARAM_MASK].as_signed_integer().unwrap_or(0);
    if char_code == UNSET && vk == UNSET {
        return None;
    }
    Some(KeyCombo {
        modifiers: decode_modifiers(mask as u64),
        key: decode_key(char_code, vk),
    })
}

fn decode_modifiers(mask: u64) -> Modifiers {
    let mut m = Modifiers::empty();
    if mask & NS_CMD != 0 {
        m |= Modifiers::CMD;
    }
    if mask & NS_OPT != 0 {
        m |= Modifiers::OPT;
    }
    if mask & NS_CTRL != 0 {
        m |= Modifiers::CTRL;
    }
    if mask & NS_SHIFT != 0 {
        m |= Modifiers::SHIFT;
    }
    if mask & NS_FN != 0 {
        m |= Modifiers::FN;
    }
    m
}

fn decode_key(char_code: i64, vk: i64) -> Key {
    // Virtual keycode wins for keys with a canonical NamedKey, since the vk
    // is layout-independent while the char_code reflects the active layout.
    if vk != UNSET
        && (0..=u16::MAX as i64).contains(&vk)
        && let Some(named) = vk_to_named(vk as u16)
    {
        return Key::Named(named);
    }

    // Fall back to the printable character if Apple set one.
    if char_code != UNSET
        && (0..=u32::MAX as i64).contains(&char_code)
        && let Some(c) = char::from_u32(char_code as u32)
        && !c.is_control()
    {
        return Key::Char(c);
    }

    // Last resort: surface the raw vk so the caller can still see what was
    // bound, even if we don't have a name for it.
    if vk != UNSET && (0..=u16::MAX as i64).contains(&vk) {
        return Key::Virtual(vk as u16);
    }

    Key::Virtual(0)
}
