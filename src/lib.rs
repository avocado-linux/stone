//! Stone - A library and CLI for managing Avocado stones.
//!
//! This crate provides both a command-line interface and a library for working
//! with Avocado Linux build artifacts, including firmware packages and FAT images.

pub mod fat;
pub mod fwup;
pub mod log;
pub mod manifest;

// Re-export commonly used items
pub use fwup::{FwupOptions, create_firmware_package};
