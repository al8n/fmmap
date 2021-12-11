use std::path::{Path, PathBuf};
use std::time::SystemTime;
use async_trait::async_trait;
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

#[async_trait]
impl AsyncMmapFileMutExt for AsyncMemoryMmapFileMut {
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.mmap.as_mut()
    }

    noop_flush!();

    async fn truncate(&mut self, max_sz: u64) -> crate::error::Result<()> {
        self.mmap.resize(max_sz as usize, 0);
        Ok(())
    }

    async fn remove(self) -> crate::error::Result<()> {
        Ok(())
    }

    async fn close_with_truncate(self, _max_sz: i64) -> crate::error::Result<()> {
        Ok(())
    }
}

impl AsyncMemoryMmapFileMut {
    pub fn freeze(self) -> AsyncMemoryMmapFile {
        AsyncMemoryMmapFile {
            mmap: self.mmap.freeze(),
            path: self.path,
            create_at: self.create_at
        }
    }
}