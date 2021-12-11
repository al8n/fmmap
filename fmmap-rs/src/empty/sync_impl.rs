use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use crate::error::{Error, Result};
use crate::metadata::{EmptyMetaData, MetaData};
use crate::mmap_file::{MmapFileExt, MmapFileMutExt};
use crate::{MmapFileReader, MmapFileWriter};

#[derive(Default)]
pub struct EmptyMmapFile {
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
        Err(Error::InvokeEmptyMmap)
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn metadata(&self) -> Result<MetaData> {
        Ok(MetaData::empty(EmptyMetaData))
    }

    fn is_exec(&self) -> bool {
        false
    }

    fn copy_all_to_vec(&self) -> Vec<u8> {
        self.inner.to_vec()
    }

    fn copy_range_to_vec(&self, _offset: usize, _len: usize) -> Vec<u8> {
        self.inner.to_vec()
    }

    fn write_all_to_new_file<P: AsRef<Path>>(&self, _new_file_path: P) -> Result<()> {
        Err(Error::InvokeEmptyMmap)
    }

    fn write_range_to_new_file<P: AsRef<Path>>(&self, _new_file_path: P, _offset: usize, _len: usize) -> Result<()> {
        Err(Error::InvokeEmptyMmap)
    }

    fn reader(&self, _offset: usize) -> Result<MmapFileReader> {
        Err(Error::InvokeEmptyMmap)
    }

    fn range_reader(&self, _offset: usize, _len: usize) -> Result<MmapFileReader> {
        Err(Error::InvokeEmptyMmap)
    }

    fn read_exact(&self, _dst: &mut [u8], _offset: usize) -> Result<()> {
        Err(Error::InvokeEmptyMmap)
    }

    fn read_i8(&self, _offset: usize) -> Result<i8> {
        Err(Error::InvokeEmptyMmap)
    }

    fn read_u8(&self, _offset: usize) -> Result<u8> {
        Err(Error::InvokeEmptyMmap)
    }
}

impl MmapFileMutExt for EmptyMmapFile {
    #[inline(always)]
    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.inner
    }

    #[inline(always)]
    fn is_cow(&self) -> bool {
        false
    }

    #[inline(always)]
    fn bytes_mut(&mut self, _offset: usize, _sz: usize) -> Result<&mut [u8]> {
         Err(Error::InvokeEmptyMmap)
    }

    #[inline(always)]
    fn zero_range(&mut self, _start: usize, _end: usize) {}

    noop_flush!();

    #[inline(always)]
    fn truncate(&mut self, _max_sz: u64) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn remove(self) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn close_with_truncate(self, _max_sz: i64) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn writer(&mut self, _offset: usize) -> Result<MmapFileWriter> {
        Err(Error::InvokeEmptyMmap)
    }

    #[inline(always)]
    fn range_writer(&mut self, _offset: usize, _len: usize) -> Result<MmapFileWriter> {
        Err(Error::InvokeEmptyMmap)
    }

    #[inline(always)]
    fn write(&mut self, _src: &[u8], _offset: usize) -> usize { 0 }

    #[inline(always)]
    fn write_all(&mut self, _src: &[u8], _offset: usize) -> Result<()> {
        Err(Error::InvokeEmptyMmap)
    }
}
