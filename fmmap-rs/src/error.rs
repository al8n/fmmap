use std::io;

/// alias for [`Result<T, Error>`]
///
/// [`Result<T, Error>`]: structs.Error.html
pub type Result<T> = std::result::Result<T, Error>;

/// Errors in this crate.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// unexpected EOF
    #[error("unexpected EOF")]
    EOF,

    /// IO error
    #[error("IO error: {0}")]
    IO(#[from] io::Error),

    /// Truncation failed
    #[error("truncation failed: {0}")]
    TruncationFailed(String),

    /// unable to open file
    #[error("unable to open file: {0}")]
    OpenFailed(String),

    /// unable to open dir
    #[error("unable to open dir: {0}")]
    OpenDirFailed(String),

    /// flush file failed
    #[error("flush file failed: {0}")]
    FlushFailed(String),

    /// sync dir failed
    #[error("sync dir failed: {0}")]
    SyncDirFailed(String),

    /// mmap failed
    #[error("mmap failed: {0}")]
    MmapFailed(String),

    /// remmap failed
    #[error("remmap failed: {0}")]
    RemmapFailed(String),

    /// invalid range
    #[error("range start must not be greater than end: {0} <= {1}")]
    InvalidBound(usize, usize),

    /// out of range
    #[error("range end out of bounds: {0} <= {1}")]
    OutOfBound(usize, usize),

    /// call on an empty mmap file
    #[error("call on an empty mmap file")]
    InvokeEmptyMmap,

    /// not a directory
    #[cfg(not(feature = "nightly"))]
    #[error("not a directory")]
    NotADirectory,
}
