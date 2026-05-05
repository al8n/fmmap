use crate::{
  disk::MmapFileMutType,
  error::{Error, ErrorKind},
  smol::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions},
  utils::smol::{
    create_file_async, open_exist_file_async, open_or_create_file_async, open_read_only_file_async,
    sync_parent_async,
  },
  MetaData,
};

use memmapix::{Mmap, MmapMut, MmapOptions};
use smol::fs::File;
use std::path::{Path, PathBuf};

use crate::disk::remmap;

/// Run a blocking IO closure off the smol executor; mirrors the helper
/// in `mmap_file::smol_impl`.
async fn run_blocking_io<F, T>(f: F) -> T
where
  F: FnOnce() -> T + Send + 'static,
  T: Send + 'static,
{
  smol::unblock(f).await
}

/// Convert the runtime-specific async file into an owned
/// `std::fs::File` to use as the inode pin during durable delete.
///
/// Smol uses `async-fs`, which does **not** expose an `into_std`
/// equivalent — its inner `std::fs::File` is held by an `Arc<File>`
/// shared with a background `Unblock` task and cannot be unwrapped
/// without an upstream API change. The fallback is to dup the fd via
/// `fcntl_dupfd_cloexec`. Under fd pressure (EMFILE) this returns
/// Err; the caller's raw `AsyncDiskMmapFileMut::drop_remove(self)`
/// will then surface the error and the file will not be deleted.
/// The wrapper-level `AsyncMmapFileMut::drop_remove` and `remove`
/// have a `remove_on_drop = true` recovery path that lets `Drop`
/// retry the dup once fds free up.
#[cfg(unix)]
async fn extract_inode_pin(file: File) -> std::io::Result<std::fs::File> {
  use std::os::fd::{AsRawFd, BorrowedFd};
  let raw = file.as_raw_fd();
  // SAFETY: file is alive for the duration of this borrow.
  let borrowed = unsafe { BorrowedFd::borrow_raw(raw) };
  let owned = rustix::io::fcntl_dupfd_cloexec(borrowed, 0).map_err(std::io::Error::from)?;
  // dup is independent now; close the async wrapper.
  drop(file);
  Ok(std::fs::File::from(owned))
}

declare_and_impl_async_fmmap_file!("smol_async", "smol", "smol", File);

declare_and_impl_async_fmmap_file_mut!("smol_async", "smol", "smol", File, AsyncDiskMmapFile);

impl_async_fmmap_file_mut_private!(AsyncDiskMmapFileMut);

impl_async_tests!(
  "smol_async_disk",
  smol_potat::test,
  smol,
  AsyncDiskMmapFile,
  AsyncDiskMmapFileMut
);

#[cfg(test)]
mod test {
  use super::*;
  use scopeguard::defer;

  #[smol_potat::test]
  async fn test_close_with_truncate_on_empty_file() {
    let file =
      unsafe { AsyncDiskMmapFileMut::create("smol_async_disk_close_with_truncate_test.txt") }
        .await
        .unwrap();
    defer!(let _ = std::fs::remove_file("smol_async_disk_close_with_truncate_test.txt"););
    file.close_with_truncate(10).await.unwrap();

    assert_eq!(
      10,
      File::open("smol_async_disk_close_with_truncate_test.txt")
        .await
        .unwrap()
        .metadata()
        .await
        .unwrap()
        .len()
    );
  }
}
