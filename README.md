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
```

Three things claim ⌃1 here. tussle orders them by where they sit in the
keyboard pipeline and marks the one that gets the key: PixPin registered
a global hotkey, so it fires before Warp's menu item can see it, and
macOS's own "Switch to Desktop 1" exists but is switched off. `--explain`
adds where to change the winner.

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

The `Fires` column is the verdict: `yes` for the global binding that
gets the key, `contested` when several share the first layer, `app` when
only app menus claim the combo (the frontmost app handles it), `off` for
a disabled binding, blank for one that never sees the key. `--explain`
adds a `change:` line saying
where the winning binding can be changed: the System Settings section
for a system shortcut, the app's own settings for a menubar app's hotkey,
App Shortcuts for a menu item.

Some global hotkeys leave no trace anywhere: apps register them in code
(`RegisterEventHotKey`), and no file or menu shows them. `--probe` lets
the key through instead of swallowing it and watches for 800 ms which
apps come to the front or open windows and whether the input source
changed. Whatever the key does will happen.

```bash
tussle who --probe
# press ctrl+space: the input source switches, and if some app also
# reacts, it shows up as an `observed` line.
```

A reaction from an app that no source lists is marked `in no source`:
the app reacts to the key from its own code, and the app's own settings
are the only place to change that. When a system shortcut fired and such
an app reacted too, both really happened: the system shortcut blocks apps
that claim the key through their menus, not apps that merely watch
keystrokes.

### Find a free combo

```bash
tussle free ctrl+opt
```

One line per key with those modifiers: `free` when nothing is bound to
it, `app-menu` when only app menu items use it (free as a global hotkey,
at the cost of those menu items), `taken` when a global binding owns it.
Pipe through `grep free` for the usable ones.

```text
 Combo           | Status   | Owner
-----------------+----------+-----------------------------------------
 ctrl+opt+a      | free     |
 ctrl+opt+d      | app-menu | WebStorm
 ctrl+opt+e      | app-menu | WebStorm
 ctrl+opt+1      | taken    | PixPin: 截图并复制
```

### Check the setup

```bash
tussle doctor
```

Reports the Accessibility and Input Monitoring permissions without
prompting, whether the system shortcut table and the preferences plist
can be read, and how many running apps answered; each line says what a
missing piece hides and where to grant it.

### Find conflicts

```bash
tussle conflicts
```

One block per combo for a person; one tab-separated line per binding
(combo, kind, role, layer, owner, action) when piped. `wins` is the
binding that gets the key; `never fires` is everything else on that
combo; `contested` means several global bindings share the first layer.
Apps reusing ⌘W
in their own menus is not a conflict and is not listed, and neither is
macOS's own ⌥⌘Esc beating the Force Quit item in every app's Apple menu.

```text
 Combo      | Kind     | Role        | Layer         | Owner       | Action
------------+----------+-------------+---------------+-------------+----------------------------------
 cmd+space  | shadowed | wins        | system        | macOS       | Show Spotlight search
 cmd+space  | shadowed | never fires | app-menu      | Lark Helper | 表情
 ctrl+1     | shadowed | wins        | global-hotkey | PixPin      | 截图
 ctrl+1     | shadowed | never fires | app-menu      | Warp        | Left Panel: Project Explorer
 ctrl+space | shadowed | wins        | system        | macOS       | Select the previous input source
 ctrl+space | shadowed | never fires | app-menu      | Warp        | New Agent Pane
 ctrl+space | shadowed | never fires | app-menu      | WebStorm    | Basic
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

### Terminal and pipes

Output follows the usual command-line conventions. On a terminal, tables
have a header and are truncated with `…` to fit the terminal width. Piped
into another program, or with `--plain`, they are tab-separated with no
header and nothing truncated, so `cut`, `awk` and `grep` see everything.
`--json` gives the full structure. Notes and warnings go to stderr;
finding nothing is not an error and exits 0.

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
- **Input Monitoring** — only for `tussle who` interactive capture and
  `--probe`.

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
