//! Watch what happens after a keystroke goes through: which apps come to
//! the front or open windows, and whether the input source changed.
//!
//! Window ownership comes from `CGWindowListCopyWindowInfo`, which reports
//! window numbers and owning pids without Screen Recording permission
//! (only window titles need that, and they are not read).

use std::collections::HashMap;
use std::ffi::c_void;
use std::time::{Duration, Instant};

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use objc2_app_kit::{NSRunningApplication, NSWorkspace};

use crate::capture::{Reaction, ReactionKind};

/// How often to look while waiting for reactions.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// The state to compare against, taken before the key is pressed.
pub(super) struct Snapshot {
    frontmost: Option<i32>,
    /// On-screen window number -> owning pid.
    windows: HashMap<u32, i32>,
    input_source: Option<String>,
}

impl Snapshot {
    pub(super) fn take() -> Self {
        Self {
            frontmost: frontmost_pid(),
            windows: windows_by_pid(),
            input_source: input_source_id(),
        }
    }
}

/// Poll for `settle` after the key and report every app that came to the
/// front or opened windows, plus an input source change.
pub(super) fn watch(
    baseline: &Snapshot,
    settle: Duration,
) -> (Vec<Reaction>, Option<(String, String)>) {
    let own_pid = std::process::id() as i32;
    let started = Instant::now();
    let mut activated: Vec<i32> = Vec::new();
    let mut opened: HashMap<i32, usize> = HashMap::new();
    while started.elapsed() < settle {
        std::thread::sleep(POLL_INTERVAL);
        if let Some(front) = frontmost_pid()
            && Some(front) != baseline.frontmost
            && front != own_pid
            && !activated.contains(&front)
        {
            activated.push(front);
        }
        for (pid, count) in new_windows(&baseline.windows, &windows_by_pid(), own_pid) {
            let seen = opened.entry(pid).or_default();
            *seen = (*seen).max(count);
        }
    }

    let mut reactions: Vec<Reaction> = activated
        .iter()
        .map(|&pid| reaction(pid, ReactionKind::Activated))
        .collect();
    let mut by_windows: Vec<(i32, usize)> = opened.into_iter().collect();
    by_windows.sort_by_key(|(pid, count)| (std::cmp::Reverse(*count), *pid));
    reactions.extend(
        by_windows
            .into_iter()
            .map(|(pid, count)| reaction(pid, ReactionKind::NewWindows(count))),
    );

    let input_source_change = match (&baseline.input_source, input_source_id()) {
        (Some(before), Some(after)) if *before != after => Some((before.clone(), after)),
        _ => None,
    };
    (reactions, input_source_change)
}

/// Windows present now that were not in `baseline`, counted per owning
/// pid. Our own windows never count.
pub(super) fn new_windows(
    baseline: &HashMap<u32, i32>,
    now: &HashMap<u32, i32>,
    own_pid: i32,
) -> HashMap<i32, usize> {
    let mut counts = HashMap::new();
    for (window, pid) in now {
        if *pid != own_pid && !baseline.contains_key(window) {
            *counts.entry(*pid).or_default() += 1;
        }
    }
    counts
}

fn reaction(pid: i32, kind: ReactionKind) -> Reaction {
    let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid);
    Reaction {
        pid,
        app_name: app
            .as_ref()
            .and_then(|a| a.localizedName())
            .map(|s| s.to_string()),
        bundle_id: app
            .as_ref()
            .and_then(|a| a.bundleIdentifier())
            .map(|s| s.to_string()),
        kind,
    }
}

fn frontmost_pid() -> Option<i32> {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .map(|app| app.processIdentifier())
}

// CGWindowListCopyWindowInfo options, from CoreGraphics/CGWindow.h:
//   kCGWindowListOptionOnScreenOnly     = 1 << 0
//   kCGWindowListExcludeDesktopElements = 1 << 4
const ON_SCREEN_ONLY: u32 = 1 << 0;
const EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;
const NULL_WINDOW: u32 = 0;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CFArrayRef;
}

fn windows_by_pid() -> HashMap<u32, i32> {
    // SAFETY: returns a +1 CFArray of CFDictionaries, or NULL.
    let raw = unsafe {
        CGWindowListCopyWindowInfo(ON_SCREEN_ONLY | EXCLUDE_DESKTOP_ELEMENTS, NULL_WINDOW)
    };
    if raw.is_null() {
        return HashMap::new();
    }
    // SAFETY: ownership of the +1 reference passes to the wrapper.
    let list: CFArray<CFDictionary<CFString, CFType>> =
        unsafe { CFArray::wrap_under_create_rule(raw) };
    let key_number = CFString::from_static_string("kCGWindowNumber");
    let key_owner = CFString::from_static_string("kCGWindowOwnerPID");
    let mut out = HashMap::with_capacity(list.len() as usize);
    for entry in list.iter() {
        let number = entry
            .find(&key_number)
            .and_then(|v| v.downcast::<CFNumber>())
            .and_then(|n| n.to_i64());
        let owner = entry
            .find(&key_owner)
            .and_then(|v| v.downcast::<CFNumber>())
            .and_then(|n| n.to_i64());
        if let (Some(number), Some(owner)) = (number, owner) {
            out.insert(number as u32, owner as i32);
        }
    }
    out
}

// Text Input Sources, from HIToolbox/TextInputSources.h.
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCopyCurrentKeyboardInputSource() -> *const c_void;
    fn TISGetInputSourceProperty(source: *const c_void, key: CFStringRef) -> *const c_void;
    static kTISPropertyInputSourceID: CFStringRef;
}

fn input_source_id() -> Option<String> {
    // SAFETY: TISCopyCurrentKeyboardInputSource returns a +1 reference or
    // NULL; the property getter returns a borrowed CFString or NULL.
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return None;
        }
        let id = TISGetInputSourceProperty(source, kTISPropertyInputSourceID);
        let result = if id.is_null() {
            None
        } else {
            Some(CFString::wrap_under_get_rule(id as CFStringRef).to_string())
        };
        core_foundation::base::CFRelease(source);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three readers need no permission, so they must work anywhere
    /// with a window server. Window and frontmost-app counts depend on the
    /// session (empty on a headless runner), the input source never is.
    #[test]
    fn system_state_readers_do_not_fail() {
        let _ = windows_by_pid();
        let _ = frontmost_pid();
        assert!(input_source_id().is_some());
    }

    #[test]
    fn new_windows_are_counted_per_owner_and_never_our_own() {
        let baseline: HashMap<u32, i32> = [(1, 100), (2, 100), (3, 200)].into();
        let now: HashMap<u32, i32> = [(1, 100), (3, 200), (4, 300), (5, 300), (6, 999)].into();
        let counts = new_windows(&baseline, &now, 999);
        assert_eq!(counts.get(&300), Some(&2));
        assert_eq!(counts.get(&999), None);
        assert_eq!(counts.get(&100), None);
    }
}
