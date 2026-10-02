//! Shared wire-format building blocks: the only place that handles raw bytes
//! and offsets. Record structs are plain `binrw` derives built from these.

pub mod flags;
pub mod id;
pub mod raw;
pub mod string;
