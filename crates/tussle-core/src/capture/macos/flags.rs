//! `CGEventFlags` → [`Modifiers`] decoding.

use crate::Modifiers;

// Mask bits from `CoreGraphics/CGEventTypes.h` (`kCGEventFlagMask*`).
// Identical to `NSEventModifierFlag*` because Cocoa events store the
// same flags.
//   kCGEventFlagMaskShift       = 1 << 17
//   kCGEventFlagMaskControl     = 1 << 18
//   kCGEventFlagMaskAlternate   = 1 << 19  (Option)
//   kCGEventFlagMaskCommand     = 1 << 20
//   kCGEventFlagMaskSecondaryFn = 1 << 23
const FLAG_SHIFT: u64 = 1 << 17;
const FLAG_CONTROL: u64 = 1 << 18;
const FLAG_ALTERNATE: u64 = 1 << 19;
const FLAG_COMMAND: u64 = 1 << 20;
const FLAG_FUNCTION: u64 = 1 << 23;

pub(super) fn decode_cg_flags(flags: u64) -> Modifiers {
    let mut m = Modifiers::empty();
    if flags & FLAG_SHIFT != 0 {
        m |= Modifiers::SHIFT;
    }
    if flags & FLAG_CONTROL != 0 {
        m |= Modifiers::CTRL;
    }
    if flags & FLAG_ALTERNATE != 0 {
        m |= Modifiers::OPT;
    }
    if flags & FLAG_COMMAND != 0 {
        m |= Modifiers::CMD;
    }
    if flags & FLAG_FUNCTION != 0 {
        m |= Modifiers::FN;
    }
    m
}
