use std::path::{Path, PathBuf};
use std::time::SystemTime;
use bytes::{Bytes, BytesMut};
use crate::{MmapFileExt, MmapFileMutExt, MetaData};
use crate::metadata::MemoryMetaData;

define_impl_constructor_for_mmap_file!(MemoryMmapFile, "MemoryMmapFile", "");

impl_mmap_file_ext!(MemoryMmapFile);

define_and_impl_constructor_for_mmap_file_mut!(MemoryMmapFileMut, "MemoryMmapFileMut", MemoryMmapFile, "MemoryMmapFile", "MmapFileExt", "");

impl_mmap_file_ext!(MemoryMmapFileMut);

impl MmapFileMutExt for MemoryMmapFileMut {
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.mmap.as_mut()
    }

    #[inline]
    fn is_cow(&self) -> bool {
        false
    }

    noop_flush!();

    #[inline]
    fn truncate(&mut self, max_sz: u64) -> crate::error::Result<()> {
        self.mmap.resize(max_sz as usize, 0);
        Ok(())
    }

    #[inline]
    fn remove(self) -> crate::error::Result<()> {
        Ok(())
    }

    #[inline]
    fn close_with_truncate(self, _max_sz: i64) -> crate::error::Result<()> {
        Ok(())
    }
}