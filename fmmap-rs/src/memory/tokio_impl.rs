use std::path::{Path, PathBuf};
use std::time::SystemTime;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use crate::{AsyncMmapFileExt, AsyncMmapFileMutExt, MetaData};
use crate::metadata::MemoryMetaData;

define_impl_constructor_for_mmap_file!(AsyncMemoryMmapFile, "AsyncMemoryMmapFile");

impl_async_mmap_file_ext!(AsyncMemoryMmapFile);

define_and_impl_constructor_for_mmap_file_mut!(AsyncMemoryMmapFileMut, "AsyncMemoryMmapFileMut");

impl_async_mmap_file_ext!(AsyncMemoryMmapFileMut);

#[async_trait]
impl AsyncMmapFileMutExt for AsyncMemoryMmapFileMut {
    #[inline(always)]
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.mmap.as_mut()
    }

    #[inline(always)]
    fn is_cow(&self) -> bool {
        false
    }

    noop_flush!();

    #[inline(always)]
    async fn truncate(&mut self, max_sz: u64) -> crate::error::Result<()> {
        self.mmap.resize(max_sz as usize, 0);
        Ok(())
    }

    #[inline(always)]
    async fn remove(self) -> crate::error::Result<()> {
        Ok(())
    }

    #[inline(always)]
    async fn close_with_truncate(self, _max_sz: i64) -> crate::error::Result<()> {
        Ok(())
    }
}

impl AsyncMemoryMmapFileMut {
    /// Make the memory mmap file immutable
    #[inline(always)]
    pub fn freeze(self) -> AsyncMemoryMmapFile {
        AsyncMemoryMmapFile {
            mmap: self.mmap.freeze(),
            path: self.path,
            create_at: self.create_at
        }
    }
}