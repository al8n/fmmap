//! Error handling for this crate.
//!
//! As of 0.6, fmmap uses [`std::io::Error`] for all fallible operations
//! instead of a custom error type.

pub use std::io::{Error, ErrorKind};

/// Convenience alias for [`std::io::Result`].
pub type Result<T> = std::io::Result<T>;

/// Error variant returned by `try_drop_remove` on raw async types
/// (`AsyncDiskMmapFileMut`). The recoverable variant preserves
/// ownership of the wrapped value so the caller can retry; the
/// terminal variant signals that the operation got past the point of
/// no-return and surfaces just the error.
#[derive(Debug)]
pub enum DropRemoveError<T> {
  /// Failed before any destructive operation (typically `EMFILE` on
  /// smol's `fcntl_dupfd_cloexec`). The wrapped value is preserved;
  /// the caller can retry (e.g. once fd pressure subsides) by
  /// calling `try_drop_remove` again.
  Recoverable(T, Error),
  /// Failed after destructive operations began (probe, unlink, or
  /// fsync). The wrapped value is gone; treat this like a
  /// `Result<(), Error>` failure.
  Terminal(Error),
}

impl<T> DropRemoveError<T> {
  /// Discard the recovered value (if any) and return just the error.
  /// Useful for callers that don't need recovery and want to flatten
  /// to a plain `io::Error`.
  pub fn into_error(self) -> Error {
    match self {
      Self::Recoverable(_, e) => e,
      Self::Terminal(e) => e,
    }
  }
}

impl<T> From<DropRemoveError<T>> for Error {
  fn from(e: DropRemoveError<T>) -> Self {
    e.into_error()
  }
}

impl<T> core::fmt::Display for DropRemoveError<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      Self::Recoverable(_, e) => write!(f, "recoverable: {e}"),
      Self::Terminal(e) => write!(f, "terminal: {e}"),
    }
  }
}

impl<T: core::fmt::Debug> core::error::Error for DropRemoveError<T> {
  fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
    match self {
      Self::Recoverable(_, e) | Self::Terminal(e) => Some(e),
    }
  }
}
