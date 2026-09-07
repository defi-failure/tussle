# Changelog

All notable changes to this project. Generated from the commit log with
[git-cliff](https://git-cliff.org); the format follows
[Conventional Commits](https://www.conventionalcommits.org).

## 0.0.1-alpha.2 — 2026-09-07

### Added

- **core**: Model layer, scope and enabled state of bindings
- **core**: Make HotkeyIndex the scan engine with lookup, winner and conflicts
- **cli**: Add conflicts subcommand
- **cli**: Add --source to restrict which sources run
- **core**: Treat status-bar shortcuts shaped like global hotkeys as global
- **cli**: Render conflicts as blocks with wins and never-fires lines
- **core**: Read the effective system hotkey table from Carbon
- **core**: Report the Apple menu once and never against itself
- **core**: Name unlabelled system menu items after app menu titles
- **cli**: Keep unnamed macOS shortcuts out of the scan table
- Probe who reacts to a key with `who --probe`
- **core**: Say when an --app pattern matches no running app
- **cli**: Say where to change the winning binding in who
- **cli**: Add free to list unused combos for a modifier set
- Add doctor to check permissions and sources
- **cli**: Explain probe reactions that no source can account for### Fixed

- **core**: Retry apps that time out during the Accessibility walk
- **core**: Desktop-switching hotkeys are off unless the plist enables them
- **core**: Input method menus and bare keys are not global hotkeys### Changed

- **core**: Return warnings alongside bindings from Source::scan
- **cli**: Drive scan and who through HotkeyIndex
- **core**: Make the symbolic hotkey id optional
- **cli**: Output for terminals and pipes the way gh does it### Build and CI

- Create releases as drafts for manual publishing
- **deps**: Bump actions/checkout from 5 to 7 (#2)
## 0.0.1-alpha.1 — 2026-09-06

### Added

- **core**: Add Modifiers bitflags type
- **core**: Add KeyCombo data model
- **core**: Add Binding with extensible BindingSource enum
- **core**: Add HotkeyIndex append-only container
- **core**: Add ScanError type for parser failures
- **core**: Implement symbolichotkeys parser
- **cli**: Add tussle-cli with clap scan subcommand stub
- **cli**: Wire scan subcommand to symbolichotkeys parser
- **core**: Add Display impls for Modifiers, Key, NamedKey, KeyCombo
- **core**: Label known symbolic hotkey IDs with human-readable names
- **core**: Expose binding owner separately from action label
- **cli**: Render scan output as aligned table via tabled
- **cli**: Add --json flag to scan subcommand
- **core**: Merge macOS default symbolic hotkeys with user overrides
- **cli**: Add --keys flag for symbol/name rendering with TTY auto-detect
- **core**: Add AppMenuOverride source with nsuserkeyequivalents scaffold
- **core**: Implement nsuserkeyequivalents parser
- **core**: Add AppMenuItem source and accessibility module scaffold
- **core**: Add macOS Accessibility deps and is_trusted check
- **core**: Implement accessibility menu-bar walker
- **core**: Define Source trait for hotkey config parsers
- **cli**: Wire scan to all three sources via Source trait
- **core**: Normalize Apple keyboard glyphs and PUA function keys to NamedKey
- **core**: Walk AXExtrasMenuBar to capture status-bar shortcuts
- **core**: Add KeyCombo::parse for textual combo input
- **cli**: Add tussle who for text-format combo lookup
- Add tussle who interactive capture via CGEventTap
- **capture**: Live modifier echo via FlagsChanged events
- **cli**: Add --ax-timeout to override AX messaging timeout
- **core**: Add SystemAction type and classify_extended_vk helper
- Surface macOS system actions in 'tussle who' instead of vkN
- **capture**: Classify vk 0xa0 as Mission Control
- **cli**: Expose --ax-concurrency to override the parallel-scan cap
- **cli**: Wire up tracing-subscriber with -v/-vv verbosity flag
- **cli**: Extend -v verbosity to support TRACE (-vvv)
- **observability**: Emit tracing events from scan/who and accessibility
- **observability**: Emit per-source INFO in CLI loop, drop core duplicate
- **core**: Add bundle_filter to Accessibility for app-side push-down
- **core**: Add ComboToken for token-level combo matching
- **cli**: Add --key and --app filters to scan (with push-down)
- **cli**: Add --group-by to scan, default = combo
- **cli**: Auto-pipe scan table through $PAGER, with --no-pager opt-out### Fixed

- **cli**: Default --keys to names; symbols opt-in only
- **core**: Treat AXMenuItem no-command bit as cmd suppressor
- **core**: Cite Apple sources for keyboard constants and correct delete mapping
- **capture**: Wire SIGINT to stop runloop and clarify permission prompt
- **capture**: Trigger Input Monitoring TCC dialog via IOHIDRequestAccess
- **capture**: Decode ANSI letter vks and echo captured combo immediately
- **cli**: Drop redundant 'is held by' prefix from tussle who output
- **combo**: Normalize ' ' to NamedKey::Space in from_char
- **combo**: Normalize LF (0x0A) to NamedKey::Return in from_char
- **cli**: EnvFilter target is 'tussle' (binary), not 'tussle_cli'
- **accessibility**: Decode undocumented AX fn modifier bit (1<<4)
- **cli**: --app filter also matches bundle_id, not just localized owner### Performance

- **accessibility**: Skip Prohibited-policy apps when scanning
- **accessibility**: Cap per-app AX messaging timeout at 1s
- **accessibility**: Skip XPC service processes by executable path
- **accessibility**: Scan apps in parallel via thread::scope, cap 128
- **accessibility**: Skip apps with no bundleURL (catches sandboxed WebContent)
- **accessibility**: Bump default max_concurrency from 128 to 512### Changed

- **cli**: Drop --keys flag and unicode symbol rendering
- **core**: Consolidate modifier/key/combo types into combo module
- **core**: Make symbolichotkeys implement Source trait
- **core**: Make nsuserkeyequivalents implement Source trait
- **core**: Make accessibility implement Source trait
- **core**: Split combo.rs into module (modifiers/key/parse/vk)
- **core**: Split capture.rs into module with macos submod
- **core**: Split accessibility source into module with macos submod
- **cli**: Split main.rs into cli module (commands/sources/output)
- Apply clippy suggestions from Rust 1.98### Documentation

- Drop internal milestone labels from committed comments
- **core**: Drop unverified NSUserKeyEquivalents reference URL
- Add project README
- Reorder TODO and tweak GUI line
- Link LICENSE in README
- Fill in real repo URL in install instructions
- Mention how to uninstall
- Add CI badge and correct minimum Rust version
- Document installer script and prebuilt binaries
- Install instructions for the universal binary### Testing

- **core**: Add symbolichotkeys fixture and ignored parser test### Build and CI

- Declare MSRV 1.88 and package metadata across the workspace
- Drop unused crates and narrow macOS binding features
- **deps**: Bump all dependencies to their latest releases
- Add GitHub Actions workflow for fmt, clippy, test, MSRV and Intel build
- Add dependabot for cargo and github-actions updates
- Pin MSRV toolchain via input and ignore dtolnay/rust-toolchain in dependabot
- Build release binaries for both macOS targets
- Configure cargo-dist for macOS release builds
- Add cargo-dist release workflow
- Let dist tolerate dependabot edits to release.yml
- Replace cargo-dist with a universal-binary release workflow### Revert

- **cli**: Remove auto-pager (pager UX wasn't an improvement)
