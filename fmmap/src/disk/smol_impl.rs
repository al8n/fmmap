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
use smol::fs::{remove_file, File};
use std::path::{Path, PathBuf};

use crate::disk::remmap;

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
