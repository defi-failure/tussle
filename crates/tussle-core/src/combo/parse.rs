//! Textual parser for `KeyCombo`: `cmd+opt+b`, `shift+f1`, `escape`, ...

use super::{ComboParseError, Key, KeyCombo, Modifiers, NamedKey};

/// A single combo "token" for partial-match queries: either a modifier
/// (`cmd`, `shift`, …) or a key (`space`, `f1`, `a`).
///
/// Used by callers that want to ask "does this combo contain X?" without
/// caring whether X is on the modifier side or the key side. `ComboToken`
/// shares the same alias table and case-insensitivity rules as
/// [`KeyCombo::parse`], so user-facing input like `Command` and `cmd` both
/// match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComboToken {
    Modifier(Modifiers),
    Key(Key),
}

impl ComboToken {
    /// Parse a single token. Tries modifier aliases first, then falls back
    /// to the key parser (named keys, then single-character).
    pub fn parse(s: &str) -> Result<Self, ComboParseError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ComboParseError::Empty);
        }
        let lower = trimmed.to_ascii_lowercase();
        if let Ok(m) = parse_modifier(&lower) {
            return Ok(ComboToken::Modifier(m));
        }
        let key = parse_key(trimmed)?;
        Ok(ComboToken::Key(key))
    }

    /// Whether `combo` includes this token. For modifier tokens it's
    /// `combo.modifiers.contains(...)`; for key tokens it's `combo.key ==
    /// ...`.
    pub fn matches(&self, combo: &KeyCombo) -> bool {
        match self {
            ComboToken::Modifier(m) => combo.modifiers.contains(*m),
            ComboToken::Key(k) => combo.key == *k,
        }
    }
}

impl KeyCombo {
    /// Parse a textual combo such as `cmd+opt+b`, `shift+f1`, or `escape`.
    ///
    /// Rules:
    ///   - Tokens are separated by `+` and are case-insensitive.
    ///   - All but the last token must be a modifier name:
    ///     `cmd`/`command`, `opt`/`alt`/`option`, `ctrl`/`control`, `shift`,
    ///     `fn`/`globe`. The last token is the key.
    ///   - Single-character tokens become `Key::Char` (lowercased).
    ///   - Multi-character tokens must match a `NamedKey` variant name
    ///     (`space`, `return`, `f1`, `pageup`, `escape`/`esc`, ...).
    pub fn parse(s: &str) -> Result<KeyCombo, ComboParseError> {
        let raw = s.trim();
        if raw.is_empty() {
            return Err(ComboParseError::Empty);
        }
        let tokens: Vec<&str> = raw.split('+').map(str::trim).collect();
        if tokens.iter().any(|t| t.is_empty()) {
            return Err(ComboParseError::Empty);
        }

        let (key_token, modifier_tokens) = tokens.split_last().expect("non-empty per check above");

        let mut modifiers = Modifiers::empty();
        for token in modifier_tokens {
            modifiers |= parse_modifier(token)?;
        }

        let key = parse_key(key_token)?;
        Ok(KeyCombo { modifiers, key })
    }
}

fn parse_modifier(token: &str) -> Result<Modifiers, ComboParseError> {
    Ok(match token.to_ascii_lowercase().as_str() {
        "cmd" | "command" => Modifiers::CMD,
        "opt" | "alt" | "option" => Modifiers::OPT,
        "ctrl" | "control" => Modifiers::CTRL,
        "shift" => Modifiers::SHIFT,
        "fn" | "globe" => Modifiers::FN,
        _ => return Err(ComboParseError::UnknownModifier(token.to_string())),
    })
}

fn parse_key(token: &str) -> Result<Key, ComboParseError> {
    let lower = token.to_ascii_lowercase();

    if let Some(named) = parse_named(&lower) {
        return Ok(Key::Named(named));
    }

    let mut chars = token.chars();
    let first = chars.next().ok_or(ComboParseError::MissingKey)?;
    if chars.next().is_some() {
        return Err(ComboParseError::UnknownKey(token.to_string()));
    }
    Ok(Key::Char(first.to_ascii_lowercase()))
}

fn parse_named(lower: &str) -> Option<NamedKey> {
    Some(match lower {
        "space" => NamedKey::Space,
        "return" | "enter" => NamedKey::Return,
        "tab" => NamedKey::Tab,
        "escape" | "esc" => NamedKey::Escape,
        "delete" | "del" => NamedKey::Delete,
        "backspace" => NamedKey::Backspace,
        "help" => NamedKey::Help,
        "insert" | "ins" => NamedKey::Insert,
        "f1" => NamedKey::F1,
        "f2" => NamedKey::F2,
        "f3" => NamedKey::F3,
        "f4" => NamedKey::F4,
        "f5" => NamedKey::F5,
        "f6" => NamedKey::F6,
        "f7" => NamedKey::F7,
        "f8" => NamedKey::F8,
        "f9" => NamedKey::F9,
        "f10" => NamedKey::F10,
        "f11" => NamedKey::F11,
        "f12" => NamedKey::F12,
        "f13" => NamedKey::F13,
        "f14" => NamedKey::F14,
        "f15" => NamedKey::F15,
        "f16" => NamedKey::F16,
        "f17" => NamedKey::F17,
        "f18" => NamedKey::F18,
        "f19" => NamedKey::F19,
        "f20" => NamedKey::F20,
        "up" => NamedKey::Up,
        "down" => NamedKey::Down,
        "left" => NamedKey::Left,
        "right" => NamedKey::Right,
        "pageup" => NamedKey::PageUp,
        "pagedown" => NamedKey::PageDown,
        "home" => NamedKey::Home,
        "end" => NamedKey::End,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_letter_with_cmd() {
        let c = KeyCombo::parse("cmd+b").unwrap();
        assert_eq!(c.modifiers, Modifiers::CMD);
        assert_eq!(c.key, Key::Char('b'));
    }

    #[test]
    fn parses_named_key_with_modifiers() {
        let c = KeyCombo::parse("ctrl+opt+space").unwrap();
        assert_eq!(c.modifiers, Modifiers::CTRL | Modifiers::OPT);
        assert_eq!(c.key, Key::Named(NamedKey::Space));
    }

    #[test]
    fn parses_bare_named_key() {
        let c = KeyCombo::parse("escape").unwrap();
        assert!(c.modifiers.is_empty());
        assert_eq!(c.key, Key::Named(NamedKey::Escape));
    }

    #[test]
    fn parse_is_case_insensitive() {
        assert_eq!(
            KeyCombo::parse("CMD+SHIFT+F1").unwrap(),
            KeyCombo::parse("cmd+shift+f1").unwrap(),
        );
    }

    #[test]
    fn parse_accepts_modifier_aliases() {
        let c = KeyCombo::parse("alt+option+command+a").unwrap();
        assert_eq!(c.modifiers, Modifiers::OPT | Modifiers::CMD);
        assert_eq!(c.key, Key::Char('a'));
    }

    #[test]
    fn parse_modifier_order_doesnt_matter() {
        assert_eq!(
            KeyCombo::parse("shift+cmd+3").unwrap(),
            KeyCombo::parse("cmd+shift+3").unwrap(),
        );
    }

    #[test]
    fn parse_round_trips_through_display() {
        let c = KeyCombo::parse("ctrl+shift+cmd+3").unwrap();
        assert_eq!(format!("{c}"), "ctrl+shift+cmd+3");
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(matches!(KeyCombo::parse(""), Err(ComboParseError::Empty)));
        assert!(matches!(KeyCombo::parse("  "), Err(ComboParseError::Empty)));
        assert!(matches!(
            KeyCombo::parse("cmd+"),
            Err(ComboParseError::Empty)
        ));
    }

    #[test]
    fn parse_rejects_unknown_modifier() {
        assert!(matches!(
            KeyCombo::parse("super+a"),
            Err(ComboParseError::UnknownModifier(_))
        ));
    }

    #[test]
    fn parse_rejects_multichar_unnamed_key() {
        assert!(matches!(
            KeyCombo::parse("cmd+ab"),
            Err(ComboParseError::UnknownKey(_))
        ));
    }

    mod combo_token {
        use super::*;

        #[test]
        fn parse_modifier_aliases() {
            assert_eq!(
                ComboToken::parse("cmd").unwrap(),
                ComboToken::Modifier(Modifiers::CMD)
            );
            assert_eq!(
                ComboToken::parse("Command").unwrap(),
                ComboToken::Modifier(Modifiers::CMD)
            );
            assert_eq!(
                ComboToken::parse("ALT").unwrap(),
                ComboToken::Modifier(Modifiers::OPT)
            );
            assert_eq!(
                ComboToken::parse("globe").unwrap(),
                ComboToken::Modifier(Modifiers::FN)
            );
        }

        #[test]
        fn parse_named_key() {
            assert_eq!(
                ComboToken::parse("space").unwrap(),
                ComboToken::Key(Key::Named(NamedKey::Space))
            );
            assert_eq!(
                ComboToken::parse("F1").unwrap(),
                ComboToken::Key(Key::Named(NamedKey::F1))
            );
            assert_eq!(
                ComboToken::parse("ESC").unwrap(),
                ComboToken::Key(Key::Named(NamedKey::Escape))
            );
        }

        #[test]
        fn parse_single_char_key() {
            assert_eq!(
                ComboToken::parse("a").unwrap(),
                ComboToken::Key(Key::Char('a'))
            );
            assert_eq!(
                ComboToken::parse("A").unwrap(),
                ComboToken::Key(Key::Char('a'))
            );
            assert_eq!(
                ComboToken::parse("3").unwrap(),
                ComboToken::Key(Key::Char('3'))
            );
        }

        #[test]
        fn parse_rejects_empty_and_unknown() {
            assert!(ComboToken::parse("").is_err());
            assert!(ComboToken::parse("   ").is_err());
            assert!(ComboToken::parse("notakey").is_err());
        }

        #[test]
        fn matches_modifier_token() {
            let cmd_a = KeyCombo {
                modifiers: Modifiers::CMD,
                key: Key::Char('a'),
            };
            let shift_cmd_a = KeyCombo {
                modifiers: Modifiers::CMD | Modifiers::SHIFT,
                key: Key::Char('a'),
            };
            assert!(ComboToken::Modifier(Modifiers::CMD).matches(&cmd_a));
            assert!(!ComboToken::Modifier(Modifiers::SHIFT).matches(&cmd_a));
            assert!(ComboToken::Modifier(Modifiers::SHIFT).matches(&shift_cmd_a));
        }

        #[test]
        fn matches_key_token() {
            let cmd_space = KeyCombo {
                modifiers: Modifiers::CMD,
                key: Key::Named(NamedKey::Space),
            };
            assert!(ComboToken::Key(Key::Named(NamedKey::Space)).matches(&cmd_space));
            assert!(!ComboToken::Key(Key::Named(NamedKey::F1)).matches(&cmd_space));
            assert!(!ComboToken::Key(Key::Char('a')).matches(&cmd_space));
        }
    }
}
