use std::fs::Metadata;
use std::os::windows::fs::MetadataExt;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::{Error, Result};
use crate::metadata::{DiskMetaData, EmptyMetaData, MemoryMetaData};

#[enum_dispatch]
pub trait MetaDataExt {
    fn accessed(&self) -> std::result::Result<SystemTime, Error>;
    fn created(&self) -> std::result::Result<SystemTime, Error>;
    fn is_file(&self) -> bool;
    #[cfg(feature = "nightly")]
    fn is_symlink(&self) -> bool;
    fn len(&self) -> u64;
    fn modified(&self) -> std::result::Result<SystemTime, Error>;

    fn file_attributes(&self) -> u32;
    fn creation_time(&self) -> u64;
    fn last_access_time(&self) -> u64;
    fn last_write_time(&self) -> u64;
    fn file_size(&self) -> u64;
    fn volume_serial_number(&self) -> Option<u32>;
    fn number_of_links(&self) -> Option<u32>;
    fn file_index(&self) -> Option<u64>;
}

impl MetaDataExt for MemoryMetaData {
    fn accessed(&self) -> Result<SystemTime> {
        Ok(self.create_at)
    }

    #[inline]
    fn created(&self) -> Result<SystemTime> {
        Ok(self.create_at)
    }

    fn is_file(&self) -> bool {
        false
    }

    #[cfg(feature = "nightly")]
    fn is_symlink(&self) -> bool {
        false
    }

    #[inline]
    fn len(&self) -> u64 {
        self.size
    }

    fn modified(&self) -> Result<SystemTime> {
        Ok(self.create_at)
    }

    fn file_attributes(&self) -> u32 {
        0
    }

    fn creation_time(&self) -> u64 {
        self.create_at.duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    fn last_access_time(&self) -> u64 {
        self.create_at.duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    fn last_write_time(&self) -> u64 {
        self.create_at.duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    fn file_size(&self) -> u64 {
        self.size
    }

    fn volume_serial_number(&self) -> Option<u32> {
        None
    }

    fn number_of_links(&self) -> Option<u32> {
        None
    }

    fn file_index(&self) -> Option<u64> {
        None
    }
}

impl MetaDataExt for DiskMetaData {
    fn accessed(&self) -> Result<SystemTime> {
        self.inner.accessed().map_err(Error::IO)
    }

    fn created(&self) -> Result<SystemTime> {
        self.inner.created().map_err(Error::IO)
    }

    fn is_file(&self) -> bool {
        self.inner.is_file()
    }

    #[cfg(feature = "nightly")]
    fn is_symlink(&self) -> bool {
        self.inner.is_symlink()
    }

    fn len(&self) -> u64 {
        self.inner.len()
    }

    fn modified(&self) -> Result<SystemTime> {
        self.inner.modified().map_err(Error::IO)
    }

    fn file_attributes(&self) -> u32 {
        self.inner.file_attributes()
    }

    fn creation_time(&self) -> u64 {
        self.inner.creation_time()
    }

    fn last_access_time(&self) -> u64 {
        self.inner.last_access_time()
    }

    fn last_write_time(&self) -> u64 {
        self.inner.last_write_time()
    }

    fn file_size(&self) -> u64 {
        self.inner.file_size()
    }

    fn volume_serial_number(&self) -> Option<u32> {
        self.inner.volume_serial_number()
    }

    fn number_of_links(&self) -> Option<u32> {
        self.inner.number_of_links()
    }

    fn file_index(&self) -> Option<u64> {
        self.inner.file_index()
    }
}

impl MetaDataExt for EmptyMetaData {
    fn accessed(&self) -> Result<SystemTime> {
        Ok(UNIX_EPOCH)
    }

    fn created(&self) -> Result<SystemTime> {
        Ok(UNIX_EPOCH)
    }

    fn is_file(&self) -> bool {
        false
    }

    #[cfg(feature = "nightly")]
    fn is_symlink(&self) -> bool {
        false
    }

    fn len(&self) -> u64 {
        0
    }

    fn modified(&self) -> Result<SystemTime> {
        Ok(UNIX_EPOCH)
    }

    fn file_attributes(&self) -> u32 {
        0
    }

    fn creation_time(&self) -> u64 {
        0
    }

    fn last_access_time(&self) -> u64 {
        0
    }

    fn last_write_time(&self) -> u64 {
        0
    }

    fn file_size(&self) -> u64 {
        0
    }

    fn volume_serial_number(&self) -> Option<u32> {
        None
    }

    fn number_of_links(&self) -> Option<u32> {
        None
    }

    fn file_index(&self) -> Option<u64> {
        None
    }
}