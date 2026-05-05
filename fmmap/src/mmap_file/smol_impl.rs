use std::{
  borrow::Cow,
  mem,
  path::{Path, PathBuf},
};

use crate::{
  disk::smol_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut},
  empty::smol_impl::AsyncEmptyMmapFile,
  error::{Error, ErrorKind, Result},
  memory::smol_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut},
  metadata::MetaData,
  smol::{AsyncMmapFileReader, AsyncMmapFileWriter, AsyncOptions},
  utils::smol::sync_dir_async,
};
use smol::io::{AsyncWriteExt, Cursor};

/// Run a blocking IO closure off the smol executor (`smol::unblock`).
/// Used by the durable-delete path so `unlinkat` + `fsync` don't stall
/// the executor.
async fn run_blocking_io<F, T>(f: F) -> T
where
  F: FnOnce() -> T + Send + 'static,
  T: Send + 'static,
{
  smol::unblock(f).await
}

/// Extract the inode pin used by the durable-delete path from an
/// async file. See `mmap_file::tokio_impl::extract_pin_or_err`.
///
/// smol: `async-fs::File` exposes no `into_std` equivalent (its inner
/// `std::fs::File` is held by an `Arc<File>` shared with a background
/// `Unblock` task). Falls back to `fcntl_dupfd_cloexec`. On EMFILE
/// the original file is returned in `Err((file, e))` so the caller
/// can restore `self.inner` and let the `remove_on_drop` fallback
/// retry via Drop.
#[cfg(unix)]
async fn extract_pin_or_err(
  file: ::smol::fs::File,
) -> std::result::Result<std::fs::File, (::smol::fs::File, Error)> {
  use std::os::fd::{AsRawFd, BorrowedFd};
  let raw = file.as_raw_fd();
  // SAFETY: file is alive for the duration of this borrow.
  let borrowed = unsafe { BorrowedFd::borrow_raw(raw) };
  match rustix::io::fcntl_dupfd_cloexec(borrowed, 0) {
    Ok(owned) => {
      drop(file);
      Ok(std::fs::File::from(owned))
    }
    Err(e) => Err((file, std::io::Error::from(e))),
  }
}

/// Synchronous variant for `impl_drop!`'s `remove_on_drop` path.
/// async-fs has no into_std equivalent and Drop can't await, so we
/// still dup. EMFILE leaves the file undeleted (Drop is best-effort);
/// same documented limitation as the smol raw async drop_remove.
#[cfg(unix)]
fn sync_drop_pin(file: ::smol::fs::File) -> Option<std::fs::File> {
  use std::os::fd::{AsRawFd, BorrowedFd};
  let raw = file.as_raw_fd();
  // SAFETY: file is alive for the duration of the borrow.
  let borrowed = unsafe { BorrowedFd::borrow_raw(raw) };
  rustix::io::fcntl_dupfd_cloexec(borrowed, 0)
    .ok()
    .map(std::fs::File::from)
}
#[cfg(not(unix))]
fn sync_drop_pin(file: ::smol::fs::File) -> Option<std::fs::File> {
  drop(file);
  None
}

declare_async_mmap_file_ext!(
  AsyncDiskMmapFileMut,
  AsyncOptions,
  AsyncMmapFileReader<'_>,
  ::smol::fs::OpenOptions
);

declare_async_mmap_file_mut_ext!(AsyncMmapFileWriter<'_>);

declare_and_impl_inners!();

declare_and_impl_async_mmap_file!("smol_async", "smol", "smol");

delcare_and_impl_async_mmap_file_mut!("smol_async", "smol", "smol");

impl_async_tests!(
  "smol_async",
  smol_potat::test,
  smol,
  AsyncMmapFile,
  AsyncMmapFileMut
);
