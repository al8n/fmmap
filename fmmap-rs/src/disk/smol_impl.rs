use std::path::{Path, PathBuf};
#[cfg(not(target_os = "linux"))]
use std::ptr::{drop_in_place, write};
use async_trait::async_trait;
use crate::MetaData;
use crate::smol::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
use crate::disk::MmapFileMutType;
use crate::error::Error;
use crate::utils::smol::{create_file_async, open_exist_file_with_append_async, open_or_create_file_async, open_read_only_file_async, sync_parent_async};
use fs4::smol::AsyncFileExt;
use memmap2::{Mmap, MmapMut, MmapOptions, MmapAsRawDesc};
use smol::fs::{File, remove_file};

remmap!(Path);

declare_and_impl_async_fmmap_file!("smol_async", "smol", "smol", File);

declare_and_impl_async_fmmap_file_mut!("smol_async", "smol", "smol", File, AsyncDiskMmapFile);

impl_async_fmmap_file_mut_private!(AsyncDiskMmapFileMut);

#[cfg(test)]
mod test {
    use super::*;
    use scopeguard::defer;

    #[smol_potat::test]
    async fn test_close_with_truncate_on_empty_file() {
        let file = AsyncDiskMmapFileMut::create("smol_async_disk_close_with_truncate_test.txt").await.unwrap();
        defer!(std::fs::remove_file("smol_async_disk_close_with_truncate_test.txt").unwrap());
        file.close_with_truncate(10).await.unwrap();

        assert_eq!(10, File::open("smol_async_disk_close_with_truncate_test.txt").await.unwrap().metadata().await.unwrap().len());
    }
}