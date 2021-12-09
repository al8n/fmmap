cfg_windows!(
    mod windows;
    pub use windows::MetaDataExt;
);

cfg_unix!(
    mod unix;
    pub use unix::MetaDataExt;
);

use std::fs::Metadata;
use std::ops::{Deref, DerefMut};
use std::time::SystemTime;
use crate::error::Error;

#[derive(Default, Copy, Clone)]
pub struct EmptyMetaData;

#[derive(Copy, Clone)]
pub struct MemoryMetaData {
    size: u64,
    create_at: SystemTime,
}

impl MemoryMetaData {
    pub(crate) fn new(size: u64, create_at: SystemTime) -> Self {
        Self {
            size,
            create_at
        }
    }
}

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

#[enum_dispatch(MetaDataExt)]
pub enum MetaData {
    Empty(EmptyMetaData),
    Memory(MemoryMetaData),
    Disk(DiskMetaData),
}

impl MetaData {
    pub(crate) fn empty(meta: EmptyMetaData) -> Self {
        Self::Empty(meta)
    }

    pub(crate) fn memory(meta: MemoryMetaData) -> Self {
        Self::Memory(meta)
    }

    pub(crate) fn disk(meta: Metadata) -> Self {
        Self::Disk(DiskMetaData::new(meta))
    }
}


