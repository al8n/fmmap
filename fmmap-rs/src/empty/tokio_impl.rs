use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use crate::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncMmapFileReader, AsyncMmapFileWriter, MetaData};
use crate::error::{Error, Result};
use crate::metadata::EmptyMetaData;

#[derive(Default, Clone)]
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

    fn is_exec(&self) -> bool {
        false
    }

    async fn metadata(&self) -> Result<MetaData> {
        Ok(MetaData::empty(EmptyMetaData))
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
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.inner
    }

    #[inline]
    fn is_cow(&self) -> bool {
        false
    }

    #[inline]
    fn bytes_mut(&mut self, _offset: usize, _len: usize) -> Result<&mut [u8]> {
        Err(Error::InvokeEmptyMmap)
    }

    #[inline]
    fn zero_range(&mut self, _start: usize, _end: usize) {}

    noop_flush!();

    #[inline]
    async fn truncate(&mut self, _max_sz: u64) -> Result<()> {
        Ok(())
    }

    #[inline]
    async fn remove(self) -> Result<()> {
        Ok(())
    }

    #[inline]
    async fn close_with_truncate(self, _max_sz: i64) -> Result<()> {
        Ok(())
    }

    #[inline]
    fn writer(&mut self, _offset: usize) -> Result<AsyncMmapFileWriter> {
        Err(Error::InvokeEmptyMmap)
    }

    #[inline]
    fn range_writer(&mut self, _offset: usize, _len: usize) -> Result<AsyncMmapFileWriter> {
        Err(Error::InvokeEmptyMmap)
    }

    #[inline]
    fn write(&mut self, _src: &[u8], _offset: usize) -> usize { 0 }

    #[inline]
    fn write_all(&mut self, _src: &[u8], _offset: usize) -> Result<()> {
        Err(Error::InvokeEmptyMmap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_empty() {
        let mut file = AsyncEmptyMmapFile::default();
        file.slice(0, 0);
        file.as_slice();
        file.as_mut_slice();
        file.bytes(0,0).unwrap_err();
        file.bytes_mut(0,0).unwrap_err();
        file.metadata().await.unwrap();
        file.copy_range_to_vec(0,0);
        file.copy_all_to_vec();
        file.write_all_to_new_file("test").await.unwrap_err();
        file.write_range_to_new_file("test", 0, 0).await.unwrap_err();
        assert!(!file.is_exec());
        assert!(!file.is_cow());
        assert_eq!(file.len(), 0);
        file.path();
        file.path_lossy();
        file.path_string();
        file.flush().unwrap();
        file.flush_async().unwrap();
        file.flush_range(0, 0).unwrap();
        file.flush_async_range(0, 0).unwrap();
        let mut buf = [0; 10];
        file.reader(0).unwrap_err();
        file.range_reader(0, 0).unwrap_err();
        file.read_i8(0).unwrap_err();
        file.read_u8(0).unwrap_err();
        file.read_exact(&mut buf, 0).unwrap_err();
        file.write(&buf, 0);
        file.write_all(&[0], 0).unwrap_err();
        file.writer(0).unwrap_err();
        file.range_writer(0, 0).unwrap_err();
        file.zero_range(0, 0);
        file.clone().close_with_truncate(0).await.unwrap();
        file.truncate(0).await.unwrap();
        file.clone().remove().await.unwrap();
    }
}