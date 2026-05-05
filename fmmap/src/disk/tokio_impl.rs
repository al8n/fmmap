use crate::{
  disk::MmapFileMutType,
  error::{Error, ErrorKind},
  tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions},
  utils::tokio::{
    create_file_async, open_exist_file_async, open_or_create_file_async, open_read_only_file_async,
    sync_parent_async,
  },
  MetaData,
};

use memmapix::{Mmap, MmapMut, MmapOptions};
use std::path::{Path, PathBuf};
use tokio::fs::File;

use crate::disk::remmap;

/// Run a blocking IO closure on tokio's blocking thread pool. Used by
/// the raw async durable-delete path; mirrors the helper in
/// `mmap_file::tokio_impl`.
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

/// Extract the inode pin from a runtime-specific async file. Mirrors
/// `mmap_file::tokio_impl::extract_pin_or_err` (same name, same shape)
/// so the helper API is uniform between the wrapper and raw async
/// delete paths. tokio's `into_std()` moves the underlying
/// `std::fs::File` without allocating a new fd, so the `Err` arm is
/// unreachable on tokio — but the shape is kept symmetric with smol
/// for trait-free helper-name dispatch from the disk macro.
#[cfg(unix)]
async fn extract_pin_or_err(
  file: File,
) -> std::result::Result<std::fs::File, (File, Error)> {
  Ok(file.into_std().await)
}

declare_and_impl_async_fmmap_file!("tokio_async", "tokio_test", "tokio", File);

declare_and_impl_async_fmmap_file_mut!(
  "tokio_async",
  "tokio_test",
  "tokio",
  File,
  AsyncDiskMmapFile
);

impl_async_fmmap_file_mut_private!(AsyncDiskMmapFileMut);

impl_async_tests!(
  "tokio_async_disk",
  tokio::test,
  tokio,
  AsyncDiskMmapFile,
  AsyncDiskMmapFileMut
);

#[cfg(test)]
mod test {
  use super::*;
  use scopeguard::defer;

  /// `try_drop_remove` succeeds on the happy path the same way the
  /// trait method does. Tokio's `into_std` is infallible so the
  /// `Recoverable` arm is unreachable here; this test only verifies
  /// that the inherent method exists, has the recoverable signature,
  /// and behaves correctly on the success path.
  #[tokio::test]
  async fn try_drop_remove_happy_path() {
    let path = "tokio_async_disk_try_drop_remove_test.txt";
    let file = unsafe { AsyncDiskMmapFileMut::create(path) }.await.unwrap();
    defer!(let _ = std::fs::remove_file(path););
    let result = file.try_drop_remove().await;
    assert!(result.is_ok(), "happy path returns Ok");
    assert!(!std::path::Path::new(path).exists());
  }

  #[tokio::test]
  async fn test_close_with_truncate_on_empty_file() {
    let file =
      unsafe { AsyncDiskMmapFileMut::create("tokio_async_disk_close_with_truncate_test.txt") }
        .await
        .unwrap();
    defer!(let _ = std::fs::remove_file("tokio_async_disk_close_with_truncate_test.txt"););
    file.close_with_truncate(10).await.unwrap();

    assert_eq!(
      10,
      File::open("tokio_async_disk_close_with_truncate_test.txt")
        .await
        .unwrap()
        .metadata()
        .await
        .unwrap()
        .len()
    );
  }
}
