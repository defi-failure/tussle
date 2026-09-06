# tussle

[![CI](https://github.com/defi-failure/tussle/actions/workflows/ci.yml/badge.svg)](https://github.com/defi-failure/tussle/actions/workflows/ci.yml)

**Inspect every keyboard shortcut on your Mac.**

Find which app uses a shortcut, list every binding registered on your
machine, and spot conflicts between apps.

```text
$ tussle who ctrl+1

 Owner  | Action
--------+----------------------------------
 macOS  | Switch to Desktop 1
 PixPin | ScreenShot
 Warp   | Left Panel: Agent Conversations
```

Three different things claim ⌃1 here. macOS dispatches to whichever
registered first; the others stay invisible. Knowing the full set
explains why a shortcut sometimes "doesn't work" and shows what to
disable to free a combo.

## Install

One universal binary runs on both Apple Silicon and Intel Macs. Grab it from
the [Releases](https://github.com/defi-failure/tussle/releases) page, or:

```bash
curl -L -o tussle https://github.com/defi-failure/tussle/releases/download/v0.0.1-alpha.1/tussle
chmod +x tussle
sudo mv tussle /usr/local/bin/
```

If you downloaded it with a browser instead of `curl`, macOS quarantines
the file; run `xattr -d com.apple.quarantine tussle` before the first launch.

From source (requires Rust 1.88+):

```bash
git clone https://github.com/defi-failure/tussle.git
cd tussle
cargo install --path crates/tussle-cli
```

Uninstall by deleting the binary, or `cargo uninstall tussle-cli` if you
built from source.

## Usage

### List every shortcut

```bash
tussle scan
```

A typical Mac has 1000–3000 bindings. The default ordering groups by
combo, so any shortcut claimed by more than one owner stacks
contiguously.

### Look up a single combo

```bash
tussle who cmd+w
tussle who 'shift+cmd+5'
```

Modifiers: `cmd`, `opt`, `ctrl`, `shift`, `fn` (case-insensitive,
aliases like `command` / `option` / `alt` / `control` / `globe` work).

Or interactive — omit the argument and press the actual key:

```bash
tussle who
# Press the hotkey to look up...
```

### Filter & group

```bash
tussle scan --app rustrover            # only RustRover bindings
tussle scan --key cmd                  # any shortcut containing cmd
tussle scan --key space --key f1       # space OR f1 shortcuts
tussle scan --group-by owner           # group by app instead of combo
tussle scan --json                     # JSON for piping to jq
```

`--app` matches both display name and bundle id, so `--app finder`
works on Chinese-localized macOS where Finder shows as `访达`.
Multiple `--app` or `--key` values combine as OR; different flags
combine as AND.

## Permissions

macOS prompts the first time each is needed:

- **Accessibility** — to read each running app's menu shortcuts.
- **Input Monitoring** — only for `tussle who` interactive capture.

Grant from `System Settings → Privacy & Security`, then re-run.

## Status

Early. macOS only.

## TODO

- Third-party launcher parsers: Karabiner, Raycast, BetterTouchTool,
  Hammerspoon, Keyboard Maestro.
- Homebrew tap.
- Persistent cache for instant repeat lookups.
- `tussle diff` — what changed since the last scan.
- GUI (likely a SwiftUI menu-bar app?).

## License

MIT — see [LICENSE](LICENSE).
