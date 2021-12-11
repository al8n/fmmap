use std::path::{Path, PathBuf};
use std::time::SystemTime;
use bytes::{Bytes, BytesMut};
use crate::{MmapFileExt, MmapFileMutExt, MetaData};
use crate::metadata::MemoryMetaData;

/// Use [`Bytes`] to mock a mmap, which is useful for test and in-memory storage engine.
///
/// [`Bytes`]: https://docs.rs/bytes/1.1.0/bytes/struct.Bytes.html
pub struct MemoryMmapFile {
    mmap: Bytes,
    path: PathBuf,
    create_at: SystemTime,
}

impl_mmap_file_ext!(MemoryMmapFile);

/// Use [`BytesMut`] to mock a mmap, which is useful for test and in-memory storage engine
///
/// [`BytesMut`]: https://docs.rs/bytes/1.1.0/bytes/struct.BytesMut.html
pub struct MemoryMmapFileMut {
    mmap: BytesMut,
    path: PathBuf,
    create_at: SystemTime,
}

impl_mmap_file_ext!(MemoryMmapFileMut);

impl MmapFileMutExt for MemoryMmapFileMut {
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
    fn truncate(&mut self, max_sz: u64) -> crate::error::Result<()> {
        self.mmap.resize(max_sz as usize, 0);
        Ok(())
    }

    #[inline(always)]
    fn remove(self) -> crate::error::Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn close_with_truncate(self, _max_sz: i64) -> crate::error::Result<()> {
        Ok(())
    }
}

impl MemoryMmapFileMut {
    /// Make the memory mmap file immutable
    #[inline(always)]
    pub fn freeze(self) -> MemoryMmapFile {
        MemoryMmapFile {
            mmap: self.mmap.freeze(),
            path: self.path,
            create_at: self.create_at,
        }
    }
}