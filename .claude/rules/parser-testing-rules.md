# Parser Testing Rules

Every config-source parser (symbolichotkeys plist, Karabiner JSON, Raycast
SQLite, BTT plist, Hammerspoon Lua, KM sqlite, NSUserKeyEquivalents) must be
fixture-driven. Most of these formats are undocumented or schema-unstable;
real-world samples are the ground truth, not imagined specs.

## Workflow per parser

1. First commit: drop a real-world sample into
   `crates/tussle-core/tests/fixtures/<source>/` and add an `#[ignore]`d test
   asserting the expected `Vec<Binding>` output.
2. Then implement the parser. Remove `#[ignore]` when the test passes.
3. Each new edge case (half-broken file, unknown field, schema variant) =
   new fixture + new test BEFORE the fix lands.

## Fixture naming

`<source>/<descriptor>.{plist,json,db,lua,...}`. Examples:
- `symbolichotkeys/default.plist`
- `karabiner/empty-profile.json`
- `karabiner/with-complex-modifications.json`
- `raycast/v1.62-schema.db`

## Don't

- Don't write parsers against guessed/imagined schemas. No real sample →
  don't ship the parser; get a sample first.
- Don't delete fixtures during refactor. Historical schema variants are
  regression coverage.
