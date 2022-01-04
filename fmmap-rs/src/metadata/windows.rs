use std::os::windows::fs::MetadataExt;
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

    /// Returns the value of the `dwFileAttributes` field of this metadata.
    fn file_attributes(&self) -> u32;

    /// Returns the value of the `ftCreationTime` field of this metadata.
    fn creation_time(&self) -> u64;

    /// Returns the value of the `ftLastAccessTime` field of this metadata.
    fn last_access_time(&self) -> u64;

    /// Returns the value of the `ftLastWriteTime` field of this metadata.
    fn last_write_time(&self) -> u64;

    /// Returns the value of the `nFileSize{High,Low}` fields of this metadata.
    fn file_size(&self) -> u64;

    /// Returns the value of the `dwVolumeSerialNumber` field of this metadata.
    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn volume_serial_number(&self) -> Option<u32>;

    /// Returns the value of the `nNumberOfLinks` field of this metadata.
    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn number_of_links(&self) -> Option<u32>;

    /// Returns the value of the `nFileIndex{Low,High}` fields of this metadata.
    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn file_index(&self) -> Option<u64>;
}

#[cfg(windows)]
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

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn volume_serial_number(&self) -> Option<u32> {
        None
    }

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn number_of_links(&self) -> Option<u32> {
        None
    }

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn file_index(&self) -> Option<u64> {
        None
    }
}

#[cfg(windows)]
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

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn volume_serial_number(&self) -> Option<u32> {
        self.inner.volume_serial_number()
    }

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn number_of_links(&self) -> Option<u32> {
        self.inner.number_of_links()
    }

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn file_index(&self) -> Option<u64> {
        self.inner.file_index()
    }
}

#[cfg(windows)]
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

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn volume_serial_number(&self) -> Option<u32> {
        None
    }

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn number_of_links(&self) -> Option<u32> {
        None
    }

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn file_index(&self) -> Option<u64> {
        None
    }
}

#[cfg(windows)]
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

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn volume_serial_number(&self) -> Option<u32> {
        self.inner.volume_serial_number()
    }

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn number_of_links(&self) -> Option<u32> {
        self.inner.number_of_links()
    }

    #[cfg(feature = "nightly")]
    #[cfg_attr(docsrs, doc(cfg(feature = "nightly")))]
    fn file_index(&self) -> Option<u64> {
        self.inner.file_index()
    }
}

#[cfg(test)]
mod tests {
    use std::time::UNIX_EPOCH;
    use bytes::Bytes;
    use crate::empty::EmptyMmapFile;
    use crate::{MetaDataExt, MmapFileExt, MmapFileMutExt, Options};
    use crate::raw::MemoryMmapFile;
    use crate::tests::get_random_file_path;

    #[test]
    fn test_metadata() {
        let mut file = Options::new()
            .max_size("Hello, fmmap!".len() as u64)
            .create_mmap_file_mut(get_random_file_path("txt"))
            .unwrap();
        file.set_remove_on_drop(true);
        file.write_all("Hello, fmmap!".as_bytes(), 0).unwrap();

        let meta = file.metadata().unwrap();
        meta.accessed().unwrap();
        meta.created().unwrap();
        assert!(meta.is_file());
        #[cfg(feature = "nightly")]
        assert!(!meta.is_symlink());
        assert_eq!(meta.len(), "Hello, fmmap!".len() as u64);
        assert_eq!(meta.file_size(), "Hello, fmmap!".len() as u64);
        meta.file_attributes();
        meta.creation_time();
        meta.last_access_time();
        meta.last_write_time();
        #[cfg(feature = "nightly")]
        assert!(meta.volume_serial_number().is_some());
        #[cfg(feature = "nightly")]
        assert!(meta.number_of_links().is_some());
        #[cfg(feature = "nightly")]
        assert!(meta.file_index().is_some());
    }

    #[test]
    fn test_memory_metadata() {
        let file = MemoryMmapFile::new("test.mem", Bytes::from("Hello, fmmap!"));
        let meta = file.metadata().unwrap();

        assert!(!meta.is_file());
        #[cfg(feature = "nightly")]
        assert!(!meta.is_symlink());
        assert_eq!(meta.len(), "Hello, fmmap!".len() as u64);
        assert_eq!(meta.file_size(), "Hello, fmmap!".len() as u64);
        assert_eq!(meta.file_attributes(), 0);
        assert!(meta.modified().unwrap() == meta.created().unwrap() && meta.created().unwrap() == meta.accessed().unwrap());
        assert!(meta.creation_time() == meta.last_access_time() && meta.last_access_time() == meta.last_write_time());
        #[cfg(feature = "nightly")]
        assert_eq!(meta.volume_serial_number(), None);
        #[cfg(feature = "nightly")]
        assert_eq!(meta.number_of_links(), None);
        #[cfg(feature = "nightly")]
        assert_eq!(meta.file_index(), None);
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
        assert_eq!(meta.file_attributes(), 0);
        assert_eq!(meta.creation_time(), 0);
        assert_eq!(meta.last_access_time(), 0);
        assert_eq!(meta.last_write_time(), 0);
        assert_eq!(meta.file_size(), 0);
        #[cfg(feature = "nightly")]
        assert_eq!(meta.volume_serial_number(), None);
        #[cfg(feature = "nightly")]
        assert_eq!(meta.number_of_links(), None);
        #[cfg(feature = "nightly")]
        assert_eq!(meta.file_index(), None);
    }
}