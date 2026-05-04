# Format Rules

`cargo fmt` is the final word. Run it; trust it. Don't fight it with
`#[rustfmt::skip]` for hand-tuned alignment.

`cargo fmt --check` must pass. CI will eventually enforce this.

## How rustfmt treats trailing comments (verified empirically)

Two different behaviors depending on context:

  - **Match arms with trailing comments** — rustfmt **auto-aligns** them
    when the arm widths are similar. If one arm is meaningfully longer
    than the rest (e.g. `Key::Named(NamedKey::PageDown)` vs sibling
    `Key::Named(NamedKey::Home)`), fmt does NOT pad the shorter arms to
    match the outlier — the outlier's `//` ends up further right and the
    block looks broken. In that case, drop the trailing comments on that
    block and use a leading-line block header instead.

  - **Item-level `const` / `let` with trailing comments** — rustfmt
    **collapses** all whitespace before `//` to a single space and does NOT
    align across declarations. For groups of related constants where
    alignment matters, use a **leading-line block header** above the group.

## Examples

Match arm — let fmt align:

```rust
match vk {
    0x31 => NamedKey::Space,     // kVK_Space
    0x33 => NamedKey::Backspace, // kVK_Delete
    0x75 => NamedKey::Delete,    // kVK_ForwardDelete
    ...
}
```

Item-level constants — leading-line block, no trailing alignment:

```rust
// NSEventModifierFlag bits, from AppKit/NSEvent.h:
//   NSEventModifierFlagShift   = 1 << 17
//   NSEventModifierFlagControl = 1 << 18
//   NSEventModifierFlagOption  = 1 << 19
//   NSEventModifierFlagCommand = 1 << 20
const NS_SHIFT: u64 = 1 << 17;
const NS_CTRL: u64 = 1 << 18;
const NS_OPT: u64 = 1 << 19;
const NS_CMD: u64 = 1 << 20;
```

## General preference

Default to **leading-line** comments for explanatory text. Use trailing only
for short eye-markers (≤ 4-5 words) and only where rustfmt's behavior makes
them readable (i.e. match arms).
