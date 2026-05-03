//! Per-source hotkey config parsers.
//!
//! Each submodule reads one specific format and produces `Vec<Binding>`.

pub mod accessibility;
pub mod nsuserkeyequivalents;
pub mod symbolichotkeys;
