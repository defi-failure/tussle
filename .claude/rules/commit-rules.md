# Commit Rules

## Cadence

Commit each logical step the moment you finish it. Never write a batch of files
and then commit them at the end — not as one big commit, not as fake
"incremental" commits split after the fact. Every commit must reflect the
repo's real state at that moment.

A logical step = one new file's single purpose, one type/fn/module, one fix,
one refactor, one feature slice. Not "today's work" or "a whole feature done".

Exceptions, only two: (1) the change is genuinely indivisible (must compile
together); (2) user explicitly says "commit these together".

Always stage explicit file paths (no `git add .` / `-A` / `--all`).

## Message format (Conventional Commits)

`<type>(<scope>)[!]: <description>`

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `chore`, `build`,
`ci`, `style`, `revert`.

Scope: optional, lowercase, affected crate/module (e.g. `core`, `cli`).
Omit for project-wide changes.

Description: imperative mood, lowercase, no trailing period, ≤ 72 chars.

Breaking change: append `!` after type/scope, OR add `BREAKING CHANGE:` footer.

Examples:
- `feat(core): add KeyCombo type`
- `fix(cli): handle missing plist gracefully`
- `feat(core)!: rename Modifiers to ModifierMask`
- `chore: scaffold cargo workspace`
