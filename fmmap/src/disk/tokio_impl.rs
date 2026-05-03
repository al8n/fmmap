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
use tokio::fs::{remove_file, File};

use crate::disk::remmap;

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
