#[cfg(windows)]
#[cfg_attr(docsrs, doc(cfg(windows)))]
mod windows;
#[cfg(windows)]
#[cfg_attr(docsrs, doc(cfg(windows)))]
pub use windows::MetaDataExt;

#[cfg(unix)]
#[cfg_attr(docsrs, doc(cfg(unix)))]
mod unix;
#[cfg(unix)]
#[cfg_attr(docsrs, doc(cfg(unix)))]
pub use unix::MetaDataExt;

use crate::error::Error;
use std::{
  fs::Metadata,
  ops::{Deref, DerefMut},
  time::SystemTime,
};

/// Empty MetaData
#[derive(Default, Copy, Clone)]
pub struct EmptyMetaData;

/// MetaData for [`MemoryMmapFile`]/[`MemoryMmapFileMut`]
///
/// [`MemoryMmapFile`]: structs.MemoryMmapFile.html
/// [`MemoryMmapFileMut`]: structs.MemoryMmapFileMut.html
#[derive(Copy, Clone)]
pub struct MemoryMetaData {
  size: u64,
  create_at: SystemTime,
}

impl MemoryMetaData {
  #[allow(dead_code)]
  pub(crate) fn new(size: u64, create_at: SystemTime) -> Self {
    Self { size, create_at }
  }
}

/// MetaData for [`DiskMmapFile`]/[`DiskMmapFileMut`]
///
/// [`DiskMmapFile`]: structs.DiskMmapFile.html
/// [`DiskMmapFileMut`]: structs.DiskMmapFileMut.html
#[derive(Clone)]
#[repr(transparent)]
pub struct DiskMetaData {
  inner: Metadata,
}

impl DiskMetaData {
  #[allow(dead_code)]
  pub(crate) fn new(meta: Metadata) -> Self {
    Self { inner: meta }
  }
}

impl Deref for DiskMetaData {
  type Target = Metadata;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl DerefMut for DiskMetaData {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

/// Metadata information about a file.
/// This structure is returned from the metadata or
/// symlink_metadata function or method and represents
/// known metadata about a file such as its permissions, size, modification times, etc
#[repr(transparent)]
pub struct MetaData {
  inner: MetaDataInner,
}

#[enum_dispatch(MetaDataExt)]
enum MetaDataInner {
  Empty(EmptyMetaData),
  Memory(MemoryMetaData),
  Disk(DiskMetaData),
}

impl MetaData {
  #[allow(dead_code)]
  pub(crate) fn empty(meta: EmptyMetaData) -> Self {
    Self {
      inner: MetaDataInner::Empty(meta),
    }
  }

  #[allow(dead_code)]
  pub(crate) fn memory(meta: MemoryMetaData) -> Self {
    Self {
      inner: MetaDataInner::Memory(meta),
    }
  }

  #[allow(dead_code)]
  pub(crate) fn disk(meta: Metadata) -> Self {
    Self {
      inner: MetaDataInner::Disk(DiskMetaData::new(meta)),
    }
  }
}
