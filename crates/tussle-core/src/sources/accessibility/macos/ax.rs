//! Thin wrappers around `AXUIElementCopyAttributeValue` for the three
//! attribute shapes we read: opaque element, string, integer, child array.

use std::ffi::c_void;
use std::ptr;

use accessibility_sys::{
    AXError, AXUIElementCopyAttributeValue, AXUIElementRef, kAXChildrenAttribute, kAXErrorSuccess,
};
use core_foundation::array::CFArray;
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;

pub(super) fn copy_attribute(element: AXUIElementRef, attribute: &str) -> Option<AXUIElementRef> {
    let attr = CFString::new(attribute);
    let mut value: CFTypeRef = ptr::null();
    let err: AXError =
        unsafe { AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value) };
    if err != kAXErrorSuccess || value.is_null() {
        return None;
    }
    Some(value as AXUIElementRef)
}

pub(super) fn copy_children(element: AXUIElementRef) -> Option<CFArray<*const c_void>> {
    let attr = CFString::new(kAXChildrenAttribute);
    let mut value: CFTypeRef = ptr::null();
    let err =
        unsafe { AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value) };
    if err != kAXErrorSuccess || value.is_null() {
        return None;
    }
    Some(unsafe { CFArray::wrap_under_create_rule(value as _) })
}

pub(super) fn copy_string(element: AXUIElementRef, attribute: &str) -> Option<String> {
    let attr = CFString::new(attribute);
    let mut value: CFTypeRef = ptr::null();
    let err =
        unsafe { AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value) };
    if err != kAXErrorSuccess || value.is_null() {
        return None;
    }
    let s = unsafe { CFString::wrap_under_create_rule(value as _) };
    Some(s.to_string())
}

pub(super) fn copy_i64(element: AXUIElementRef, attribute: &str) -> Option<i64> {
    let attr = CFString::new(attribute);
    let mut value: CFTypeRef = ptr::null();
    let err =
        unsafe { AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value) };
    if err != kAXErrorSuccess || value.is_null() {
        return None;
    }
    let n = unsafe { CFNumber::wrap_under_create_rule(value as _) };
    n.to_i64()
}
