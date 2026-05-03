use crate::{
  error::{Error, ErrorKind, Result},
  metadata::{EmptyMetaData, MetaData},
  mmap_file::{MmapFileExt, MmapFileMutExt},
  MmapFileReader, MmapFileWriter,
};
use std::path::{Path, PathBuf};

#[derive(Default, Clone)]
pub struct EmptyMmapFile {
  inner: [u8; 0],
  path: PathBuf,
}

impl MmapFileExt for EmptyMmapFile {
  fn len(&self) -> usize {
    0
  }

  fn as_slice(&self) -> &[u8] {
    &self.inner
  }

  fn bytes(&self, _offset: usize, _sz: usize) -> Result<&[u8]> {
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
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
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
  }

  fn write_range_to_new_file<P: AsRef<Path>>(
    &self,
    _new_file_path: P,
    _offset: usize,
    _len: usize,
  ) -> Result<()> {
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
  }

  fn reader(&self, _offset: usize) -> Result<MmapFileReader<'_>> {
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
  }

  fn range_reader(&self, _offset: usize, _len: usize) -> Result<MmapFileReader<'_>> {
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
  }

  noop_file_lock!();

  fn read_exact(&self, _dst: &mut [u8], _offset: usize) -> Result<()> {
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
  }

  fn read_i8(&self, _offset: usize) -> Result<i8> {
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
  }

  fn read_u8(&self, _offset: usize) -> Result<u8> {
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
  }
}

impl MmapFileMutExt for EmptyMmapFile {
  #[inline]
  fn as_mut_slice(&mut self) -> &mut [u8] {
    &mut self.inner
  }

  #[inline]
  fn is_cow(&self) -> bool {
    false
  }

  #[inline]
  fn bytes_mut(&mut self, _offset: usize, _sz: usize) -> Result<&mut [u8]> {
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
  }

  #[inline]
  fn zero_range(&mut self, _start: usize, _end: usize) {}

  noop_flush!();

  #[inline]
  fn truncate(&mut self, _max_sz: u64) -> Result<()> {
    Ok(())
  }

  #[inline]
  fn drop_remove(self) -> Result<()> {
    Ok(())
  }

  #[inline]
  fn close_with_truncate(self, _max_sz: i64) -> Result<()> {
    Ok(())
  }

  #[inline]
  fn writer(&mut self, _offset: usize) -> Result<MmapFileWriter<'_>> {
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
  }

  #[inline]
  fn range_writer(&mut self, _offset: usize, _len: usize) -> Result<MmapFileWriter<'_>> {
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
  }

  #[inline]
  fn write(&mut self, _src: &[u8], _offset: usize) -> usize {
    0
  }

  #[inline]
  fn write_all(&mut self, _src: &[u8], _offset: usize) -> Result<()> {
    Err(Error::new(
      ErrorKind::Unsupported,
      "call on an empty mmap file",
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_empty() {
    let mut file = EmptyMmapFile::default();
    file.slice(0, 0);
    file.as_slice();
    file.as_mut_slice();
    file.bytes(0, 0).unwrap_err();
    file.bytes_mut(0, 0).unwrap_err();
    file.metadata().unwrap();
    file.copy_range_to_vec(0, 0);
    file.copy_all_to_vec();
    file.write_all_to_new_file("test").unwrap_err();
    file.write_range_to_new_file("test", 0, 0).unwrap_err();
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
    file.clone().close_with_truncate(0).unwrap();
    file.truncate(0).unwrap();
    file.clone().drop_remove().unwrap();
  }
}
