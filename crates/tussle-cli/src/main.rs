use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tussle_core::sources::symbolichotkeys;
use tussle_core::{Binding, BindingSource, Key, KeyCombo, Modifiers, NamedKey};

#[derive(Parser)]
#[command(name = "tussle", version, about = "macOS hotkey conflict resolver")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan all hotkey sources and print discovered bindings.
    Scan {
        /// Emit JSON instead of a human-readable table.
        #[arg(long)]
        json: bool,

        /// How to render key combos in non-JSON output.
        #[arg(long, value_enum, default_value_t = KeyStyle::Names)]
        keys: KeyStyle,
    },
}

/// How key combos are rendered in human-readable output.
///
/// Default is `names` because terminal fonts often render the macOS keyboard
/// symbols (⌘⌥⌃⇧⎋⏎...) at uneven widths, breaking column alignment and
/// readability. `symbols` and `both` remain available for users with fonts
/// that handle them well (SF Mono, Nerd Fonts, etc.).
#[derive(Clone, Copy, Debug, ValueEnum)]
enum KeyStyle {
    /// Lowercase names: `cmd+shift+3`.
    Names,
    /// Unicode symbols: `⌘⇧3`. May render unevenly in some terminal fonts.
    Symbols,
    /// Both forms: `⌘⇧3 (shift+cmd+3)`.
    Both,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan { json, keys } => scan(json, keys),
    }
}

fn scan(as_json: bool, keys: KeyStyle) -> Result<()> {
    let path = system_symbolichotkeys_path()?;
    let bindings = symbolichotkeys::scan(&path)
        .with_context(|| format!("scanning {}", path.display()))?;

    if as_json {
        let rows: Vec<BindingJson> = bindings.iter().map(BindingJson::from).collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if bindings.is_empty() {
        println!("(no customized system shortcuts found)");
        return Ok(());
    }

    let mut builder = Builder::default();
    builder.push_record(["Combo", "Owner", "Action"]);
    for b in &bindings {
        builder.push_record([&render_combo(&b.combo, keys), b.source.owner(), &b.label]);
    }
    println!("{}", builder.build().with(Style::psql()));
    Ok(())
}

fn system_symbolichotkeys_path() -> Result<PathBuf> {
    Ok(dirs::preference_dir()
        .context("could not locate user preferences directory")?
        .join("com.apple.symbolichotkeys.plist"))
}

fn render_combo(c: &KeyCombo, style: KeyStyle) -> String {
    match style {
        KeyStyle::Names => format!("{c}"),
        KeyStyle::Symbols => combo_symbols(c),
        KeyStyle::Both => format!("{} ({})", combo_symbols(c), c),
    }
}

fn combo_symbols(c: &KeyCombo) -> String {
    format!("{}{}", modifier_symbols(c.modifiers), key_symbol(&c.key))
}

/// Modifier glyphs in macOS visual order: 🌐 ⌃ ⌥ ⇧ ⌘.
fn modifier_symbols(m: Modifiers) -> String {
    let mut s = String::new();
    if m.contains(Modifiers::FN) {
        s.push_str("🌐");
    }
    if m.contains(Modifiers::CTRL) {
        s.push('⌃');
    }
    if m.contains(Modifiers::OPT) {
        s.push('⌥');
    }
    if m.contains(Modifiers::SHIFT) {
        s.push('⇧');
    }
    if m.contains(Modifiers::CMD) {
        s.push('⌘');
    }
    s
}

fn key_symbol(k: &Key) -> String {
    use NamedKey::*;
    match k {
        // Capitalize letters (⌘C, not ⌘c); digits and punctuation pass through.
        Key::Char(c) => c.to_uppercase().collect(),

        Key::Named(Space) => "␣".into(),
        Key::Named(Return) => "⏎".into(),
        Key::Named(Tab) => "⇥".into(),
        Key::Named(Escape) => "⎋".into(),
        Key::Named(Backspace) => "⌫".into(),
        Key::Named(Delete) => "⌦".into(),
        Key::Named(Up) => "↑".into(),
        Key::Named(Down) => "↓".into(),
        Key::Named(Left) => "←".into(),
        Key::Named(Right) => "→".into(),
        Key::Named(PageUp) => "⇞".into(),
        Key::Named(PageDown) => "⇟".into(),
        Key::Named(Home) => "↖".into(),
        Key::Named(End) => "↘".into(),

        // F-keys, Help, Insert: no widely-recognized Unicode glyph.
        Key::Named(other) => format!("{other}").to_uppercase(),

        Key::Virtual(v) => format!("vk{v}"),
    }
}

#[derive(Serialize)]
struct BindingJson<'a> {
    combo: String,
    owner: &'static str,
    action: &'a str,
    source: SourceJson,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SourceJson {
    SystemSymbolicHotkey { id: u32 },
}

impl<'a> From<&'a Binding> for BindingJson<'a> {
    fn from(b: &'a Binding) -> Self {
        Self {
            combo: format!("{}", b.combo),
            owner: b.source.owner(),
            action: &b.label,
            source: match &b.source {
                BindingSource::SystemSymbolicHotkey { id } => {
                    SourceJson::SystemSymbolicHotkey { id: *id }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_symbols_in_macos_order() {
        let m = Modifiers::CMD | Modifiers::SHIFT | Modifiers::CTRL;
        assert_eq!(modifier_symbols(m), "⌃⇧⌘");
    }

    #[test]
    fn modifier_symbols_includes_fn_first() {
        let m = Modifiers::FN | Modifiers::CMD;
        assert_eq!(modifier_symbols(m), "🌐⌘");
    }

    #[test]
    fn key_symbol_uppercases_char() {
        assert_eq!(key_symbol(&Key::Char('c')), "C");
    }

    #[test]
    fn key_symbol_named_space() {
        assert_eq!(key_symbol(&Key::Named(NamedKey::Space)), "␣");
    }

    #[test]
    fn key_symbol_arrow() {
        assert_eq!(key_symbol(&Key::Named(NamedKey::Up)), "↑");
    }

    #[test]
    fn key_symbol_function_key_stays_text() {
        assert_eq!(key_symbol(&Key::Named(NamedKey::F1)), "F1");
    }

    #[test]
    fn render_combo_symbols_concatenates() {
        let c = KeyCombo {
            modifiers: Modifiers::CMD,
            key: Key::Named(NamedKey::Space),
        };
        assert_eq!(render_combo(&c, KeyStyle::Symbols), "⌘␣");
    }

    #[test]
    fn render_combo_names_uses_display() {
        let c = KeyCombo {
            modifiers: Modifiers::CMD | Modifiers::SHIFT,
            key: Key::Char('3'),
        };
        assert_eq!(render_combo(&c, KeyStyle::Names), "shift+cmd+3");
    }

    #[test]
    fn render_combo_both_shows_symbols_and_names() {
        let c = KeyCombo {
            modifiers: Modifiers::CMD,
            key: Key::Char('c'),
        };
        assert_eq!(render_combo(&c, KeyStyle::Both), "⌘C (cmd+c)");
    }
}
