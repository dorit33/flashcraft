//! FlashCraft core: pure, wasm-safe data model.
//!
//! This crate depends on nothing in the workspace and builds for
//! `wasm32-unknown-unknown`. All device I/O lives in `flashcraft-native`.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

pub mod checksum;
pub mod commands;
pub mod drive;
pub mod error;
pub mod image;
pub mod plan;
pub mod progress;

pub use checksum::{hash_bytes, Checksums, MultiHasher};
pub use commands::{command, registry, CommandSpec};
pub use drive::DriveInfo;
pub use error::{Error, Result};
pub use image::{ImageInfo, ImageKind};
pub use plan::{
    is_supported_compression, parse_cluster_bytes, validate_write_params, FileSystem, FormatPlan,
    PartitionScheme, WriteParams, WritePlan,
};
pub use progress::{Phase, Progress};
