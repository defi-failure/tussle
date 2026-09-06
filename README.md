# tussle

[![CI](https://github.com/defi-failure/tussle/actions/workflows/ci.yml/badge.svg)](https://github.com/defi-failure/tussle/actions/workflows/ci.yml)

**Inspect every keyboard shortcut on your Mac.**

Find which app uses a shortcut, list every binding registered on your
machine, and spot conflicts between apps.

```text
$ tussle who ctrl+1

 Fires | Layer         | Owner  | Action
-------+---------------+--------+------------------------------
 yes   | global-hotkey | PixPin | 截图
       | app-menu      | Warp   | Left Panel: Project Explorer
 off   | system        | macOS  | Switch to Desktop 1

ctrl+1 fires PixPin (global-hotkey layer): 截图; the other binding never sees this key
```

Three things claim ⌃1 here. tussle orders them by where they sit in the
keyboard pipeline: PixPin registered a global hotkey, so it takes the key
before Warp's menu item can see it, and macOS's own "Switch to Desktop 1"
exists but ships switched off. Change or disable the winner and the next
one down starts working.

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

### Find conflicts

```bash
tussle conflicts
```

Lists every combo where bindings get in each other's way: two global
bindings on one combo, or a global binding sitting on a combo that apps
also use in their menus. "wins" is the binding that gets the key; "never
fires" is everything else on that combo. In the first block below
Spotlight keeps working and it is Lark's emoji item that is dead. Several
apps reusing ⌘W in their own menus is not a conflict and is not listed,
and neither is macOS's own ⌥⌘Esc beating the Force Quit item in every
app's Apple menu: that is one function reachable twice.

```text
cmd+space  global beats app menu
  wins         macOS: Show Spotlight search  [system]
  never fires  Lark Helper: 表情  [app-menu]

ctrl+1  global beats app menu
  wins         PixPin: 截图  [global-hotkey]
  never fires  Warp: Left Panel: Project Explorer  [app-menu]

ctrl+space  global beats app menu
  wins         macOS: Select the previous input source  [system]
  never fires  Warp: New Agent Pane  [app-menu]
               WebStorm: Basic  [app-menu]
```

### Filter & group

```bash
tussle scan --app rustrover            # only RustRover bindings
tussle scan --key cmd                  # any shortcut containing cmd
tussle scan --key space --key f1       # space OR f1 shortcuts
tussle scan --group-by owner           # group by app instead of combo
tussle scan --json                     # JSON for piping to jq
tussle scan --source symbolichotkeys   # skip the app menu walk; instant
```

System shortcuts come from the table macOS itself enforces (Carbon's
`CopySymbolicHotKeys`), so ⌘Tab, ⌥⌘Esc, ⌃⌘Space and the fn/Globe key
shortcuts are known even though none of them has an entry in
`com.apple.symbolichotkeys.plist`, and the enabled state is the real
one rather than an assumed default. Shortcuts macOS only offers as
standard menu items in every app (Minimize, Fill, Center) are reported on
the `app-menu` layer, so they never count as beating an app. Entries
nobody could name are left out of the `scan` table and kept in `--json`.

`--app` matches both display name and bundle id, so `--app finder`
works on Chinese-localized macOS where Finder shows as `访达`.
Multiple `--app` or `--key` values combine as OR; different flags
combine as AND. `--source` (repeatable) restricts which sources run:
`symbolichotkeys`, `nsuserkeyequivalents`, `accessibility`.

Every command accepts `--json`. Rows carry the `layer` a binding sits on
(`system`, `global-hotkey`, `app-menu`, and more as sources are added)
and whether it is `enabled`.

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
- Global hotkey detection. A status-bar menu shortcut that looks like a
  global hotkey (⌃-based, fn-based, ⌘-less, or a function key) is
  reported on the `global-hotkey` layer; ordinary ⌘-plus-key items stay
  `app-menu`. Confirming an actual registration needs a different source.
- Homebrew tap.
- Persistent cache for instant repeat lookups.
- `tussle diff` — what changed since the last scan.
- GUI (likely a SwiftUI menu-bar app?).

## License

MIT — see [LICENSE](LICENSE).
