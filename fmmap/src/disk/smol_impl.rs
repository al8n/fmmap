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

/// Extract the inode pin from a runtime-specific async file. Mirrors
/// `mmap_file::smol_impl::extract_pin_or_err` (same name, same shape).
///
/// Smol uses `async-fs`, which does **not** expose an `into_std`
/// equivalent — its inner `std::fs::File` is held by an `Arc<File>`
/// shared with a background `Unblock` task and cannot be unwrapped
/// without upstream API support (see smol-rs/async-fs#56). The
/// fallback is to dup the fd via `fcntl_dupfd_cloexec`. Under fd
/// pressure (EMFILE) the dup fails and the file is returned in
/// `Err((file, error))` so the caller (raw `drop_remove` or wrapper
/// `drop_remove`/`remove`) can decide whether to surface the error,
/// reconstruct itself, or trigger a retry.
#[cfg(unix)]
async fn extract_pin_or_err(
  file: File,
) -> std::result::Result<std::fs::File, (File, Error)> {
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

  /// `try_drop_remove` succeeds on the happy path. The recoverable
  /// arm (smol-EMFILE on the dup) requires fault injection that's
  /// not exercised in CI; see #15.
  #[smol_potat::test]
  async fn try_drop_remove_happy_path() {
    let path = "smol_async_disk_try_drop_remove_test.txt";
    let file = unsafe { AsyncDiskMmapFileMut::create(path) }.await.unwrap();
    defer!(let _ = std::fs::remove_file(path););
    let result = file.try_drop_remove().await;
    assert!(result.is_ok(), "happy path returns Ok");
    assert!(!std::path::Path::new(path).exists());
  }

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
