use std::os::unix::fs::MetadataExt;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::{Error, Result};
use crate::MetaData;
use crate::metadata::{DiskMetaData, EmptyMetaData, MemoryMetaData};

/// Utility methods to MetaData
#[enum_dispatch]
pub trait MetaDataExt {
    /// Returns the last access time of this metadata.
    ///
    /// The returned value corresponds to the atime field of stat on Unix platforms and the ftLastAccessTime field on Windows platforms.
    ///
    /// Note that not all platforms will keep this field update in a file’s metadata,
    /// for example Windows has an option to disable updating
    /// this time when files are accessed and Linux similarly has noatime.
    fn accessed(&self) -> std::result::Result<SystemTime, Error>;

    /// Returns the creation time listed in this metadata.
    ///
    /// The returned value corresponds to the `btime` field of `statx` on Linux kernel starting from to 4.11,
    /// the `birthtime` field of stat on other Unix platforms,
    /// and the `ftCreationTime` field on Windows platforms.
    fn created(&self) -> std::result::Result<SystemTime, Error>;

    /// Returns true if this metadata is for a regular file.
    ///
    /// It will be false for symlink metadata obtained from [`symlink_metadata`].
    ///
    /// When the goal is simply to read from (or write to) the source,
    /// the most reliable way to test the source can be read (or written to) is to open it.
    /// Only using is_file can break workflows like diff <( prog_a ) on a Unix-like system for example.
    ///
    /// [`symlink_metadata`]: https://doc.rust-lang.org/std/fs/fn.symlink_metadata.html
    fn is_file(&self) -> bool;

    /// Returns `true` if this metadata is for a symbolic link.
    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn is_symlink(&self) -> bool;

    /// Returns the size of the file, in bytes, this metadata is for.
    fn len(&self) -> u64;

    /// Returns the last modification time listed in this metadata.
    ///
    /// The returned value corresponds to the `mtime` field of `stat` on Unix platforms
    /// and the `ftLastWriteTime` field on Windows platforms.
    ///
    /// # Errors
    /// This field might not be available on all platforms, and
    /// will return an `Err` on platforms where it is not available.
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
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
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
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
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

impl MetaDataExt for MetaData {
    fn accessed(&self) -> std::result::Result<SystemTime, Error> {
        self.inner.accessed()
    }

    fn created(&self) -> std::result::Result<SystemTime, Error> {
        self.inner.created()
    }

    fn is_file(&self) -> bool {
        self.inner.is_file()
    }

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn is_symlink(&self) -> bool {
        self.inner.is_symlink()
    }

    fn len(&self) -> u64 {
        self.inner.len()
    }

    fn modified(&self) -> std::result::Result<SystemTime, Error> {
        self.inner.modified()
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


#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use crate::empty::EmptyMmapFile;
    use crate::{MmapFileExt, MmapFileMutExt, Options};
    use crate::raw::MemoryMmapFile;
    use crate::tests::get_random_filename;
    use super::*;

    macro_rules! metadata_test {
        ($expr: expr) => {
            let meta = $expr;
            meta.accessed().unwrap();
            meta.created().unwrap();
            assert!(meta.is_file());
            #[cfg(feature = "nightly")]
            assert!(!meta.is_symlink());
            assert_eq!(meta.len(), "Hello, fmmap!".len() as u64);
            assert_eq!(meta.size(), "Hello, fmmap!".len() as u64);
            meta.modified().unwrap();
            meta.dev();
            meta.ino();
            meta.mode();
            meta.nlink();
            meta.uid();
            meta.gid();
            meta.rdev();
            meta.size();
            meta.atime();
            meta.atime_nsec();
            meta.mtime();
            meta.mtime_nsec();
            meta.ctime();
            meta.ctime_nsec();
            meta.blocks();
            meta.blksize();
        };
    }

    #[test]
    fn test_metadata() {
        let mut file = Options::new()
            .max_size("Hello, fmmap!".len() as u64)
            .create_mmap_file_mut(get_random_filename())
            .unwrap();
        file.set_remove_on_drop(true);
        file.write_all("Hello, fmmap!".as_bytes(), 0).unwrap();
        metadata_test!(file.metadata().unwrap());

        // let meta = file.metadata().unwrap();
        // meta.accessed().unwrap();
        // meta.created().unwrap();
        // assert!(meta.is_file());
        // #[cfg(feature = "nightly")]
        // assert!(!meta.is_symlink());
        // assert_eq!(meta.len(), "Hello, fmmap!".len() as u64);
        // assert_eq!(meta.size(), "Hello, fmmap!".len() as u64);
        // meta.modified().unwrap();
        // meta.dev();
        // meta.ino();
        // meta.mode();
        // meta.nlink();
        // meta.uid();
        // meta.gid();
        // meta.rdev();
        // meta.size();
        // meta.atime();
        // meta.atime_nsec();
        // meta.mtime();
        // meta.mtime_nsec();
        // meta.ctime();
        // meta.ctime_nsec();
        // meta.blocks();
        // meta.blksize();
    }

    #[cfg(feature = "tokio-async")]
    #[tokio::test]
    async fn test_async_metadata() {
        use crate::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
        let mut file = AsyncOptions::new()
            .max_size("Hello, fmmap!".len() as u64)
            .create_mmap_file_mut(get_random_filename())
            .await
            .unwrap();
        file.set_remove_on_drop(true);
        file.write_all("Hello, fmmap!".as_bytes(), 0).unwrap();
        metadata_test!(file.metadata().await.unwrap());

        // let meta = file.metadata().await.unwrap();
        // meta.accessed().unwrap();
        // meta.created().unwrap();
        // assert!(meta.is_file());
        // #[cfg(feature = "nightly")]
        // assert!(!meta.is_symlink());
        // assert_eq!(meta.len(), "Hello, fmmap!".len() as u64);
        // assert_eq!(meta.size(), "Hello, fmmap!".len() as u64);
        // meta.modified().unwrap();
        // meta.dev();
        // meta.ino();
        // meta.mode();
        // meta.nlink();
        // meta.uid();
        // meta.gid();
        // meta.rdev();
        // meta.size();
        // meta.atime();
        // meta.atime_nsec();
        // meta.mtime();
        // meta.mtime_nsec();
        // meta.ctime();
        // meta.ctime_nsec();
        // meta.blocks();
        // meta.blksize();
    }

    #[test]
    fn test_memory_metadata() {
        let file = MemoryMmapFile::new("test.mem", Bytes::from("Hello, fmmap!"));
        let meta = file.metadata().unwrap();

        assert!(!meta.is_file());
        #[cfg(feature = "nightly")]
        assert!(!meta.is_symlink());
        assert_eq!(meta.len(), "Hello, fmmap!".len() as u64);
        assert_eq!(meta.size(), "Hello, fmmap!".len() as u64);
        assert!(meta.modified().unwrap() == meta.created().unwrap() && meta.created().unwrap() == meta.accessed().unwrap());
        assert!(meta.atime() == meta.mtime() && meta.mtime() == meta.ctime());
        assert!(meta.atime_nsec() == meta.mtime_nsec() && meta.mtime_nsec() == meta.ctime_nsec());
        assert_eq!(meta.dev(), 0);
        assert_eq!(meta.ino(), 0);
        assert_eq!(meta.mode(), 0);
        assert_eq!(meta.nlink(), 0);
        assert_eq!(meta.uid(), 0);
        assert_eq!(meta.gid(), 0);
        assert_eq!(meta.rdev(), 0);
        assert_eq!(meta.blocks(), 0);
        assert_eq!(meta.blksize(), 0);
    }

    #[test]
    fn test_empty_metadata() {
        let file = EmptyMmapFile::default();
        let meta = file.metadata().unwrap();

        assert_eq!(meta.accessed().unwrap(), UNIX_EPOCH);
        assert_eq!(meta.created().unwrap(), UNIX_EPOCH);
        assert!(!meta.is_file());
        #[cfg(feature = "nightly")]
        assert!(!meta.is_symlink());
        assert_eq!(meta.len(), 0);
        assert_eq!(meta.modified().unwrap(), UNIX_EPOCH);
        assert_eq!(meta.dev(), 0);
        assert_eq!(meta.ino(), 0);
        assert_eq!(meta.mode(), 0);
        assert_eq!(meta.nlink(), 0);
        assert_eq!(meta.uid(), 0);
        assert_eq!(meta.gid(), 0);
        assert_eq!(meta.rdev(), 0);
        assert_eq!(meta.size(), 0);
        assert_eq!(meta.atime(), 0);
        assert_eq!(meta.atime_nsec(), 0);
        assert_eq!(meta.mtime(), 0);
        assert_eq!(meta.mtime_nsec(), 0);
        assert_eq!(meta.ctime(), 0);
        assert_eq!(meta.ctime_nsec(), 0);
        assert_eq!(meta.blocks(), 0);
        assert_eq!(meta.blksize(), 0);
    }
}