use std::path::{Path, PathBuf};
use std::time::SystemTime;
use bytes::{Bytes, BytesMut};
use crate::{AsyncMmapFileExt, AsyncMmapFileMutExt, MetaData};
use crate::metadata::MemoryMetaData;


pub struct AsyncMemoryMmapFile {
    mmap: Bytes,
    path: PathBuf,
    create_at: SystemTime,
}

impl_async_mmap_file_ext!(AsyncMemoryMmapFile);

pub struct AsyncMemoryMmapFileMut {
    mmap: BytesMut,
    path: PathBuf,
    create_at: SystemTime,
}

impl_async_mmap_file_ext!(AsyncMemoryMmapFileMut);

impl AsyncMmapFileMutExt for AsyncMemoryMmapFileMut {
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.mmap.as_mut()
    }

    fn flush(&self) -> crate::error::Result<()> {
        Ok(())
    }

    fn flush_async(&self) -> crate::error::Result<()> {
        Ok(())
    }

    fn flush_range(&self, offset: usize, len: usize) -> crate::error::Result<()> {
        Ok(())
    }

    fn flush_async_range(&self, offset: usize, len: usize) -> crate::error::Result<()> {
        Ok(())
    }

    async fn truncate(&mut self, max_sz: u64) -> crate::error::Result<()> {
        self.mmap.resize(max_sz as usize, 0);
        Ok(())
    }

    async fn delete(self) -> crate::error::Result<()> {
        Ok(())
    }

    async fn close_with_truncate(self, max_sz: i64) -> crate::error::Result<()> {
        Ok(())
    }
}