use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use crate::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncMmapFileReader, AsyncMmapFileWriter, MetaData};
use crate::error::{Error, Result};

#[derive(Default)]
pub struct AsyncEmptyMmapFile {
    inner: [u8; 0],
    path: PathBuf,
}

impl DerefMut for AsyncEmptyMmapFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Deref for AsyncEmptyMmapFile {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AsMut<Self> for AsyncEmptyMmapFile {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl AsRef<Self> for AsyncEmptyMmapFile {
    fn as_ref(&self) -> &Self {
        self
    }
}

#[async_trait]
impl AsyncMmapFileExt for AsyncEmptyMmapFile {
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

    async fn metadata(&self) -> Result<MetaData> {
        Err(Error::InvokeEmptyMmap)
    }

    fn copy_all_to_vec(&self) -> Vec<u8> {
        self.inner.to_vec()
    }

    fn copy_range_to_vec(&self, _offset: usize, _len: usize) -> Vec<u8> {
        self.inner.to_vec()
    }

    async fn write_all_to_new_file<P: AsRef<Path> + Send>(&self, _new_file_path: P) -> Result<()> {
        Err(Error::InvokeEmptyMmap)
    }

    async fn write_range_to_new_file<P: AsRef<Path> + Send>(&self, _new_file_path: P, _offset: usize, _sz: usize) -> Result<()> {
        Err(Error::InvokeEmptyMmap)
    }

    fn reader(&self, _offset: usize) -> Result<AsyncMmapFileReader> {
        Err(Error::InvokeEmptyMmap)
    }

    fn range_reader(&self, _offset: usize, _len: usize) -> Result<AsyncMmapFileReader> {
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

#[async_trait]
impl AsyncMmapFileMutExt for AsyncEmptyMmapFile {
    #[inline(always)]
    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.inner
    }

    #[inline(always)]
    fn bytes_mut(&mut self, _offset: usize, _len: usize) -> Result<&mut [u8]> {
        Err(Error::InvokeEmptyMmap)
    }

    #[inline(always)]
    fn zero_range(&mut self, _start: usize, _end: usize) {}

    noop_flush!();

    #[inline(always)]
    async fn truncate(&mut self, _max_sz: u64) -> Result<()> {
        Err(Error::InvokeEmptyMmap)
    }

    #[inline(always)]
    async fn remove(self) -> Result<()> {
        Err(Error::InvokeEmptyMmap)
    }

    #[inline(always)]
    async fn close_with_truncate(self, _max_sz: i64) -> Result<()> {
        Err(Error::InvokeEmptyMmap)
    }

    fn writer(&mut self, _offset: usize) -> Result<AsyncMmapFileWriter> {
        Err(Error::InvokeEmptyMmap)
    }

    fn range_writer(&mut self, _offset: usize, _len: usize) -> Result<AsyncMmapFileWriter> {
        Err(Error::InvokeEmptyMmap)
    }

    #[inline(always)]
    fn write(&mut self, _src: &[u8], _offset: usize) -> usize { 0 }

    #[inline(always)]
    fn write_all(&mut self, _src: &[u8], _offset: usize) -> Result<()> {
        Err(Error::InvokeEmptyMmap)
    }
}
