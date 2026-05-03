use crate::{
  error::{Error, Result},
  metadata::{DiskMetaData, EmptyMetaData, MemoryMetaData},
  MetaData,
};
use std::{
  os::windows::fs::MetadataExt,
  time::{SystemTime, UNIX_EPOCH},
};

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
}

// `volume_serial_number`, `number_of_links` and `file_index` were intentionally
// removed: they are gated behind the unstable `windows_by_handle` feature in
// `std::os::windows::fs::MetadataExt` (tracking issue #63010) and so cannot be
// implemented on stable Rust without a `winapi`/`windows-sys` dep. Callers that
// need these can call `GetFileInformationByHandle` directly via the file
// handle they obtain from the underlying file.

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
}

#[cfg(windows)]
impl MetaDataExt for DiskMetaData {
  fn accessed(&self) -> Result<SystemTime> {
    self.inner.accessed()
  }

  fn created(&self) -> Result<SystemTime> {
    self.inner.created()
  }

  fn is_file(&self) -> bool {
    self.inner.is_file()
  }

  fn is_symlink(&self) -> bool {
    self.inner.is_symlink()
  }

  fn len(&self) -> u64 {
    self.inner.len()
  }

  fn modified(&self) -> Result<SystemTime> {
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
}

// The test module exercises the disk + memory + empty backends, all of which
// only exist when the `sync` feature is enabled. Gate the whole module so
// `cargo hack --each-feature` (which iterates a no-feature build) compiles.
#[cfg(all(test, feature = "sync"))]
mod tests {
  use crate::{
    empty::EmptyMmapFile, raw::MemoryMmapFile, tests::get_random_filename, MetaDataExt,
    MmapFileExt, MmapFileMutExt, Options,
  };
  use bytes::Bytes;
  use std::time::UNIX_EPOCH;

  #[test]
  fn test_metadata() {
    let mut file = unsafe {
      Options::new()
        .max_size("Hello, fmmap!".len() as u64)
        .create_mmap_file_mut(get_random_filename())
    }
    .unwrap();
    file.set_remove_on_drop(true);
    file.write_all("Hello, fmmap!".as_bytes(), 0).unwrap();

    let meta = file.metadata().unwrap();
    meta.accessed().unwrap();
    meta.created().unwrap();
    assert!(meta.is_file());

    assert!(!meta.is_symlink());
    assert_eq!(meta.len(), "Hello, fmmap!".len() as u64);
    assert_eq!(meta.file_size(), "Hello, fmmap!".len() as u64);
    meta.file_attributes();
    meta.creation_time();
    meta.last_access_time();
    meta.last_write_time();
  }

  #[test]
  fn test_memory_metadata() {
    let file = MemoryMmapFile::new("test.mem", Bytes::from("Hello, fmmap!"));
    let meta = file.metadata().unwrap();

    assert!(!meta.is_file());

    assert!(!meta.is_symlink());
    assert_eq!(meta.len(), "Hello, fmmap!".len() as u64);
    assert_eq!(meta.file_size(), "Hello, fmmap!".len() as u64);
    assert_eq!(meta.file_attributes(), 0);
    assert!(
      meta.modified().unwrap() == meta.created().unwrap()
        && meta.created().unwrap() == meta.accessed().unwrap()
    );
    assert!(
      meta.creation_time() == meta.last_access_time()
        && meta.last_access_time() == meta.last_write_time()
    );
  }

  #[test]
  fn test_empty_metadata() {
    let file = EmptyMmapFile::default();
    let meta = file.metadata().unwrap();

    assert_eq!(meta.accessed().unwrap(), UNIX_EPOCH);
    assert_eq!(meta.created().unwrap(), UNIX_EPOCH);
    assert!(!meta.is_file());

    assert!(!meta.is_symlink());
    assert_eq!(meta.len(), 0);
    assert_eq!(meta.modified().unwrap(), UNIX_EPOCH);
    assert_eq!(meta.file_attributes(), 0);
    assert_eq!(meta.creation_time(), 0);
    assert_eq!(meta.last_access_time(), 0);
    assert_eq!(meta.last_write_time(), 0);
    assert_eq!(meta.file_size(), 0);
  }
}
