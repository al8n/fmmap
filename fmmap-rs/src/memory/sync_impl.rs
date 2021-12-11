use std::path::{Path, PathBuf};
use std::time::SystemTime;
use bytes::{Bytes, BytesMut};
use crate::{MmapFileExt, MmapFileMutExt, MetaData};
use crate::metadata::MemoryMetaData;

pub struct MemoryMmapFile {
    mmap: Bytes,
    path: PathBuf,
    create_at: SystemTime,
}

impl_mmap_file_ext!(MemoryMmapFile);

pub struct MemoryMmapFileMut {
    mmap: BytesMut,
    path: PathBuf,
    create_at: SystemTime,
}

impl_mmap_file_ext!(MemoryMmapFileMut);

impl MmapFileMutExt for MemoryMmapFileMut {
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.mmap.as_mut()
    }

    noop_flush!();

    fn truncate(&mut self, max_sz: u64) -> crate::error::Result<()> {
        self.mmap.resize(max_sz as usize, 0);
        Ok(())
    }

    fn remove(self) -> crate::error::Result<()> {
        Ok(())
    }

    fn close_with_truncate(self, _max_sz: i64) -> crate::error::Result<()> {
        Ok(())
    }
}

impl MemoryMmapFileMut {
    pub fn freeze(self) -> MemoryMmapFile {
        MemoryMmapFile {
            mmap: self.mmap.freeze(),
            path: self.path,
            create_at: self.create_at,
        }
    }
}