use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use crate::error::Result;
use crate::metadata::{MetaData, EmptyMetaData};
use crate::mmap_file::{MmapFileExt, MmapFileMutExt};

#[derive(Default)]
pub(crate) struct EmptyMmapFile {
    inner: [u8; 0],
    path: PathBuf,
}

impl DerefMut for EmptyMmapFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Deref for EmptyMmapFile {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AsMut<Self> for EmptyMmapFile {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl AsRef<Self> for EmptyMmapFile {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl MmapFileExt for EmptyMmapFile {
    fn len(&self) -> usize {
        0
    }

    fn as_slice(&self) -> &[u8] {
        &self.inner
    }

    fn bytes(&self, _offset: usize, _sz: usize) -> Result<&[u8]> {
        Ok(&self.inner)
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn stat(&self) -> Result<MetaData> {
        Ok(MetaData::Empty(EmptyMetaData))
    }
}

impl MmapFileMutExt for EmptyMmapFile {
    #[inline(always)]
    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.inner
    }

    #[inline(always)]
    fn bytes_mut(&mut self, _offset: usize, _sz: usize) -> Result<&mut [u8]> {
        Ok(&mut self.inner)
    }

    #[inline(always)]
    fn zero_range(&mut self, _start: usize, _end: usize) {}

    #[inline(always)]
    fn flush(&self) -> crate::error::Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn flush_async(&self) -> crate::error::Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn flush_range(&self, _offset: usize, _len: usize) -> crate::error::Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn flush_async_range(&self, _offset: usize, _len: usize) -> crate::error::Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn truncate(&mut self, _max_sz: u64) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn delete(self) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn close_with_truncate(self, _max_sz: i64) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn write(&mut self, _src: &[u8], _offset: usize) -> usize {
        0
    }
}


