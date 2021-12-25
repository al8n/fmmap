cfg_windows!(
    mod windows;
    pub use windows::MetaDataExt;
);

cfg_unix!(
    mod unix;
    pub use unix::MetaDataExt;
);

use crate::error::Error;
use std::fs::Metadata;
use std::ops::{Deref, DerefMut};
use std::time::SystemTime;

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
    pub(crate) fn empty(meta: EmptyMetaData) -> Self {
        Self {
            inner: MetaDataInner::Empty(meta),
        }
    }

    pub(crate) fn memory(meta: MemoryMetaData) -> Self {
        Self {
            inner: MetaDataInner::Memory(meta),
        }
    }

    pub(crate) fn disk(meta: Metadata) -> Self {
        Self {
            inner: MetaDataInner::Disk(DiskMetaData::new(meta)),
        }
    }
}
