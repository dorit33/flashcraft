//! FlashCraft native backend. May use `flashcraft-core`; performs the actual
//! device enumeration and image I/O. Linux first: drives come from sysfs,
//! writes go through buffered block-device I/O with progress callbacks.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

pub mod linux;
pub mod ops;

pub use ops::{
    compute_checksums, format_drive, inspect_image, scan_bad_blocks, verify_image, write_image,
};
