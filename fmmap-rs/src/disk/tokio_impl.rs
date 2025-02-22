use crate::disk::MmapFileMutType;
use crate::error::{Error, ErrorKind};
use crate::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
use crate::utils::tokio::{
    create_file_async, open_exist_file_with_append_async, open_or_create_file_async,
    open_read_only_file_async, sync_parent_async,
};
use crate::MetaData;

use fs4::tokio::AsyncFileExt;
use memmap2::{Mmap, MmapAsRawDesc, MmapMut, MmapOptions};
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "linux"))]
use std::ptr::{drop_in_place, write};
use tokio::fs::{remove_file, File};

remmap!(Path);

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
        let file = AsyncDiskMmapFileMut::create("tokio_async_disk_close_with_truncate_test.txt")
            .await
            .unwrap();
        defer!(std::fs::remove_file("tokio_async_disk_close_with_truncate_test.txt").unwrap());
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
