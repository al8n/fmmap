//! Error handling for this crate.
//!
//! As of 0.6, fmmap uses [`std::io::Error`] for all fallible operations
//! instead of a custom error type.

pub use std::io::{Error, ErrorKind};

/// Convenience alias for [`std::io::Result`].
pub type Result<T> = std::io::Result<T>;
