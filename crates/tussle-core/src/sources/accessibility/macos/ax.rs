//! Thin wrappers around `AXUIElementCopyAttributeValue` for the attribute
//! shapes we read: opaque element, string, integer, child array.
//!
//! Every wrapper reports a timeout distinctly from "no such attribute", so
//! callers can stop walking an app that has stopped answering instead of
//! silently recording it as having no menus.

use std::ffi::c_void;
use std::ptr;
use std::time::{Duration, Instant};

use accessibility_sys::{
    AXError, AXUIElementCopyAttributeValue, AXUIElementRef, kAXChildrenAttribute,
    kAXErrorCannotComplete, kAXErrorSuccess,
};
use core_foundation::array::CFArray;
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;

/// Why an attribute could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AxFailure {
    /// `kAXErrorCannotComplete`, with how long the call took. macOS returns
    /// this both when the messaging timeout expired (the call took about
    /// the whole timeout) and, immediately, for processes that have no
    /// Accessibility server at all. The caller tells them apart by the
    /// elapsed time.
    CannotComplete(Duration),
    /// Any other failure, including "attribute unsupported" and "no
    /// value", which are normal for elements that simply lack the
    /// attribute.
    Other,
}

fn copy_value(element: AXUIElementRef, attribute: &str) -> Result<CFTypeRef, AxFailure> {
    let attr = CFString::new(attribute);
    let mut value: CFTypeRef = ptr::null();
    let started = Instant::now();
    let err: AXError =
        unsafe { AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value) };
    if err == kAXErrorCannotComplete {
        return Err(AxFailure::CannotComplete(started.elapsed()));
    }
    if err != kAXErrorSuccess || value.is_null() {
        return Err(AxFailure::Other);
    }
    Ok(value)
}

pub(super) fn copy_attribute(
    element: AXUIElementRef,
    attribute: &str,
) -> Result<AXUIElementRef, AxFailure> {
    Ok(copy_value(element, attribute)? as AXUIElementRef)
}

pub(super) fn copy_children(element: AXUIElementRef) -> Result<CFArray<*const c_void>, AxFailure> {
    let value = copy_value(element, kAXChildrenAttribute)?;
    Ok(unsafe { CFArray::wrap_under_create_rule(value as _) })
}

pub(super) fn copy_string(element: AXUIElementRef, attribute: &str) -> Result<String, AxFailure> {
    let value = copy_value(element, attribute)?;
    let s = unsafe { CFString::wrap_under_create_rule(value as _) };
    Ok(s.to_string())
}

pub(super) fn copy_i64(element: AXUIElementRef, attribute: &str) -> Result<i64, AxFailure> {
    let value = copy_value(element, attribute)?;
    let n = unsafe { CFNumber::wrap_under_create_rule(value as _) };
    n.to_i64().ok_or(AxFailure::Other)
}
