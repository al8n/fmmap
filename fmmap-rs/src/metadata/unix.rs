use std::os::unix::fs::MetadataExt;
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

    /// Returns the ID of the device containing the file.
    fn dev(&self) -> u64;

    /// Returns the inode number.
    fn ino(&self) -> u64;

    /// Returns the rights applied to this file.
    fn mode(&self) -> u32;

    /// Returns the number of hard links pointing to this file.
    fn nlink(&self) -> u64;

    /// Returns the user ID of the owner of this file.
    fn uid(&self) -> u32;

    /// Returns the group ID of the owner of this file.
    fn gid(&self) -> u32;

    /// Returns the device ID of this file (if it is a special one).
    fn rdev(&self) -> u64;

    /// Returns the total size of this file in bytes.
    fn size(&self) -> u64;

    /// Returns the last access time of the file, in seconds since Unix Epoch.
    fn atime(&self) -> i64;

    /// Returns the last access time of the file, in nanoseconds since atime.
    fn atime_nsec(&self) -> i64;

    /// Returns the last modification time of the file, in seconds since Unix Epoch.
    fn mtime(&self) -> i64;

    /// Returns the last modification time of the file, in nanoseconds since mtime.
    fn mtime_nsec(&self) -> i64;

    /// Returns the last status change time of the file, in seconds since Unix Epoch.
    fn ctime(&self) -> i64;

    /// Returns the last status change time of the file, in nanoseconds since ctime.
    fn ctime_nsec(&self) -> i64;

    /// Returns the block size for filesystem I/O.
    fn blksize(&self) -> u64;

    /// Returns the number of blocks allocated to the file, in 512-byte units.
    ///
    /// Please note that this may be smaller than st_size / 512 when the file has holes.
    fn blocks(&self) -> u64;
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

    fn dev(&self) -> u64 {
        0
    }

    fn ino(&self) -> u64 {
        0
    }

    fn mode(&self) -> u32 {
        0
    }

    fn nlink(&self) -> u64 {
        0
    }

    fn uid(&self) -> u32 {
        0
    }

    fn gid(&self) -> u32 {
        0
    }

    fn rdev(&self) -> u64 {
        0
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn atime(&self) -> i64 {
        self.create_at.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
    }

    fn atime_nsec(&self) -> i64 {
        self.create_at.duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64
    }

    fn mtime(&self) -> i64 {
        self.create_at.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
    }

    fn mtime_nsec(&self) -> i64 {
        self.create_at.duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64
    }

    fn ctime(&self) -> i64 {
        self.create_at.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
    }

    fn ctime_nsec(&self) -> i64 {
        self.create_at.duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64
    }

    fn blksize(&self) -> u64 {
        0
    }

    fn blocks(&self) -> u64 {
        0
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

    fn dev(&self) -> u64 {
        self.inner.dev()
    }

    fn ino(&self) -> u64 {
        self.inner.ino()
    }

    fn mode(&self) -> u32 {
        self.inner.mode()
    }

    fn nlink(&self) -> u64 {
        self.inner.nlink()
    }

    fn uid(&self) -> u32 {
        self.inner.uid()
    }

    fn gid(&self) -> u32 {
        self.inner.gid()
    }

    fn rdev(&self) -> u64 {
        self.inner.rdev()
    }

    fn size(&self) -> u64 {
        self.inner.size()
    }

    fn atime(&self) -> i64 {
        self.inner.atime()
    }

    fn atime_nsec(&self) -> i64 {
        self.inner.atime_nsec()
    }

    fn mtime(&self) -> i64 {
        self.inner.mtime()
    }

    fn mtime_nsec(&self) -> i64 {
        self.inner.mtime_nsec()
    }

    fn ctime(&self) -> i64 {
        self.inner.ctime()
    }

    fn ctime_nsec(&self) -> i64 {
        self.inner.ctime_nsec()
    }

    fn blksize(&self) -> u64 {
        self.inner.blksize()
    }

    fn blocks(&self) -> u64 {
        self.inner.blocks()
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

    fn dev(&self) -> u64 {
        0
    }

    fn ino(&self) -> u64 {
        0
    }

    fn mode(&self) -> u32 {
        0
    }

    fn nlink(&self) -> u64 {
        0
    }

    fn uid(&self) -> u32 {
        0
    }

    fn gid(&self) -> u32 {
        0
    }

    fn rdev(&self) -> u64 {
        0
    }

    fn size(&self) -> u64 {
        0 
    }

    fn atime(&self) -> i64 {
        0
    }

    fn atime_nsec(&self) -> i64 {
        0
    }

    fn mtime(&self) -> i64 {
        0
    }

    fn mtime_nsec(&self) -> i64 {
        0
    }

    fn ctime(&self) -> i64 {
        0
    }

    fn ctime_nsec(&self) -> i64 {
        0
    }

    fn blksize(&self) -> u64 {
        0
    }

    fn blocks(&self) -> u64 {
        0
    }
}