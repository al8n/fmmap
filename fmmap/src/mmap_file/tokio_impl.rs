use crate::{
  disk::tokio_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut},
  empty::tokio_impl::AsyncEmptyMmapFile,
  error::{Error, ErrorKind, Result},
  memory::tokio_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut},
  metadata::MetaData,
  tokio::{AsyncMmapFileReader, AsyncMmapFileWriter, AsyncOptions},
  utils::tokio::sync_dir_async,
};
use std::{
  borrow::Cow,
  io::Cursor,
  mem,
  path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

/// Run a blocking IO closure on tokio's blocking thread pool. Used by
/// the durable-delete path to keep `unlinkat` + `fsync` off the async
/// executor thread. If the closure panics, the panic is propagated
/// (matches the behavior the caller would see for a synchronous call).
async fn run_blocking_io<F, T>(f: F) -> T
where
  F: FnOnce() -> T + Send + 'static,
  T: Send + 'static,
{
  match tokio::task::spawn_blocking(f).await {
    Ok(r) => r,
    Err(e) => std::panic::resume_unwind(e.into_panic()),
  }
}

/// Extract the inode pin used by the durable-delete path from an
/// async file, returning the original file on failure so the wrapper
/// can restore `self.inner` and let the caller retry.
///
/// tokio: `tokio::fs::File::into_std` moves the underlying
/// `std::fs::File` out without allocating a new fd. Infallible for
/// fd-pressure reasons — `drop_remove(self)` and `remove(&mut self)`
/// cannot fail with EMFILE on tokio.
#[cfg(unix)]
async fn extract_pin_or_err(
  file: ::tokio::fs::File,
) -> std::result::Result<std::fs::File, (::tokio::fs::File, Error)> {
  Ok(file.into_std().await)
}

/// Synchronous variant of the inode-pin extraction used by
/// `impl_drop!`'s `remove_on_drop` path. Drop runs in a sync context
/// — we can't `await` here. tokio's `try_into_std` is sync and
/// returns the inner `std::fs::File` directly when there are no
/// in-flight ops; we fall back to `fcntl_dupfd_cloexec` if it does
/// (rare for a wrapper at Drop time). On EMFILE the file leaks
/// (Drop is best-effort); same constraint as the smol raw path.
#[cfg(unix)]
fn sync_drop_pin(file: ::tokio::fs::File) -> Option<std::fs::File> {
  match file.try_into_std() {
    Ok(std_file) => Some(std_file),
    Err(orig) => {
      use std::os::fd::{AsRawFd, BorrowedFd};
      let raw = orig.as_raw_fd();
      // SAFETY: orig is alive for the duration of the borrow.
      let borrowed = unsafe { BorrowedFd::borrow_raw(raw) };
      rustix::io::fcntl_dupfd_cloexec(borrowed, 0)
        .ok()
        .map(std::fs::File::from)
    }
  }
}
#[cfg(not(unix))]
fn sync_drop_pin(file: ::tokio::fs::File) -> Option<std::fs::File> {
  drop(file);
  None
}

declare_async_mmap_file_ext!(
  AsyncDiskMmapFileMut,
  AsyncOptions,
  AsyncMmapFileReader<'_>,
  ::tokio::fs::OpenOptions
);

declare_async_mmap_file_mut_ext!(AsyncMmapFileWriter<'_>);

declare_and_impl_inners!();

declare_and_impl_async_mmap_file!("tokio_async", "tokio_test", "tokio");

delcare_and_impl_async_mmap_file_mut!("tokio_async", "tokio_test", "tokio");

impl_async_tests!(
  "tokio_async",
  tokio::test,
  tokio,
  AsyncMmapFile,
  AsyncMmapFileMut
);
