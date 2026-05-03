use crate::{
  disk::MmapFileMutType,
  error::{Error, ErrorKind},
  options::Options,
  utils::{create_file, open_exist_file, open_or_create_file, open_read_only_file, sync_parent},
  MetaData, MmapFileExt, MmapFileMutExt,
};
use memmapix::{Mmap, MmapMut, MmapOptions};
use std::{
  fs::{remove_file, File},
  path::{Path, PathBuf},
};

use crate::disk::remmap;

fn sync_file_and_parent(file: &File, path: &Path) -> Result<(), Error> {
  file.sync_all()?;
  sync_parent(path)
}

/// DiskMmapFile contains an immutable mmap buffer
/// and a read-only file.
pub struct DiskMmapFile {
  pub(crate) mmap: Mmap,
  pub(crate) file: File,
  pub(crate) path: PathBuf,
  exec: bool,
  /// Tracks the current advisory file-lock state so the public lock
  /// methods can short-circuit when the desired lock is already held.
  /// Initialized to `LOCK_SHARED` after the constructor's auto-acquire.
  pub(crate) lock_state: u8,
}

impl_mmap_file_ext!(DiskMmapFile);

impl DiskMmapFile {
  /// Open a readable memory map backed by a file
  ///
  /// # Examples
  ///
  /// ```ignore
  /// use fmmap::MmapFileExt;
  /// use fmmap::raw::DiskMmapFile;
  /// use std::fs::{remove_file, File};
  /// use std::io::Write;
  /// # use scopeguard::defer;
  ///
  /// # let mut file = File::create("disk_open_test.txt").unwrap();
  /// # defer!(remove_file("disk_open_test.txt").unwrap());
  /// # file.write_all("some data...".as_bytes()).unwrap();
  /// # drop(file);
  /// // open and mmap the file
  /// let mut file = DiskMmapFile::open("disk_open_test.txt").unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  /// ```
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
    Self::open_in(path, None)
  }

  /// Open a readable memory map backed by a file with [`Options`]
  ///
  /// # Examples
  ///
  /// ```ignore
  /// use fmmap::{Options, MmapFileExt};
  /// use fmmap::raw::DiskMmapFile;
  /// use std::fs::File;
  /// use std::io::Write;
  /// # use scopeguard::defer;
  ///
  /// # let mut file = File::create("disk_open_test_with_options.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_open_test_with_options.txt").unwrap());
  /// # file.write_all("sanity text".as_bytes()).unwrap();
  /// # file.write_all("some data...".as_bytes()).unwrap();
  /// # drop(file);
  ///
  /// // mmap the file with options
  /// let opts = Options::new()
  ///     // allow read
  ///     .read(true)
  ///     // allow write
  ///     .write(true)
  ///     // allow append
  ///     .append(true)
  ///     // truncate to 100
  ///     .max_size(100)
  ///     // mmap content after the sanity text
  ///     .offset("sanity text".as_bytes().len() as u64);
  /// // open and mmap the file
  /// let mut file = DiskMmapFile::open_with_options("disk_open_test_with_options.txt", opts).unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  /// ```
  ///
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
    Self::open_in(path, Some(opts))
  }

  /// Open a readable and executable memory map backed by a file
  ///
  /// # Examples
  ///
  /// ```ignore
  /// use fmmap::MmapFileExt;
  /// use fmmap::raw::DiskMmapFile;
  /// use std::fs::{remove_file, File};
  /// use std::io::Write;
  /// # use scopeguard::defer;
  ///
  /// # let mut file = File::create("disk_open_exec_test.txt").unwrap();
  /// # defer!(remove_file("disk_open_exec_test.txt").unwrap());
  /// # file.write_all("some data...".as_bytes()).unwrap();
  /// # drop(file);
  /// // open and mmap the file
  /// let mut file = DiskMmapFile::open_exec("disk_open_exec_test.txt").unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  /// ```
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open_exec<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
    Self::open_exec_in(path, None)
  }

  /// Open a readable and executable memory map backed by a file with [`Options`].
  ///
  /// # Examples
  ///
  /// ```ignore
  /// use fmmap::{Options, MmapFileExt};
  /// use fmmap::raw::DiskMmapFile;
  /// use std::fs::File;
  /// use std::io::Write;
  /// # use scopeguard::defer;
  ///
  /// # let mut file = File::create("disk_open_exec_test_with_options.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_open_exec_test_with_options.txt").unwrap());
  /// # file.write_all("sanity text".as_bytes()).unwrap();
  /// # file.write_all("some data...".as_bytes()).unwrap();
  /// # drop(file);
  ///
  /// // mmap the file with options
  /// let opts = Options::new()
  ///     // allow read
  ///     .read(true)
  ///     // mmap content after the sanity text
  ///     .offset("sanity text".as_bytes().len() as u64);
  /// // open and mmap the file
  /// let mut file = DiskMmapFile::open_exec_with_options("disk_open_exec_test_with_options.txt", opts).unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  /// ```
  ///
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open_exec_with_options<P: AsRef<Path>>(
    path: P,
    opts: Options,
  ) -> Result<Self, Error> {
    Self::open_exec_in(path, Some(opts))
  }

  fn open_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
    let path_ref = path.as_ref();
    let file = open_read_only_file(path_ref)?;
    // Auto-acquire shared lock to block conflicting writable mappings.
    <_ as fs4::FileExt>::try_lock_shared(&file).map_err(Error::from)?;

    if let Some(opts) = opts.as_ref() {
      crate::disk::validate_mapping_range(file.metadata()?.len(), opts.offset, opts.len)?;
    }
    let mmap = unsafe {
      match &opts {
        None => Mmap::map(&file)?,
        Some(opts) => opts.mmap_opts.clone().map(&file)?,
      }
    };
    Ok(Self {
      mmap,
      file,
      path: path_ref.to_path_buf(),
      exec: false,
      lock_state: crate::disk::LOCK_SHARED,
    })
  }

  fn open_exec_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
    let path_ref = path.as_ref();
    let file = open_read_only_file(path_ref)?;
    <_ as fs4::FileExt>::try_lock_shared(&file).map_err(Error::from)?;

    if let Some(opts) = opts.as_ref() {
      crate::disk::validate_mapping_range(file.metadata()?.len(), opts.offset, opts.len)?;
    }
    let mmap = unsafe {
      match &opts {
        None => MmapOptions::new().map_exec(&file)?,
        Some(opts) => opts.mmap_opts.clone().map_exec(&file)?,
      }
    };
    Ok(Self {
      mmap,
      file,
      path: path_ref.to_path_buf(),
      exec: true,
      lock_state: crate::disk::LOCK_SHARED,
    })
  }
}

/// DiskMmapFile contains a mutable mmap buffer
/// and a writable file.
pub struct DiskMmapFileMut {
  pub(crate) mmap: MmapMut,
  pub(crate) file: File,
  pub(crate) path: PathBuf,
  opts: Option<MmapOptions>,
  /// User-requested mapping offset (mirror of `opts.offset()`). Cached because
  /// `MmapOptions` doesn't expose a getter, and `truncate` needs it to clamp
  /// the new mapping against the new EOF.
  offset: u64,
  /// User-requested mapping length (mirror of `opts.len()`). `None` means
  /// "to end of file".
  len: Option<usize>,
  typ: MmapFileMutType,
  /// Set when `truncate` failed after the mapping was replaced with the
  /// anonymous placeholder. Reads return `&[]` and writes/flush/truncate
  /// return errors so the caller can't continue working with anon memory.
  poisoned: bool,
  /// See `DiskMmapFile::lock_state`. Initialized to `LOCK_EXCLUSIVE`
  /// after the constructor's auto-acquire (or `LOCK_SHARED` for COW
  /// mappings where the constructor took a shared lock).
  pub(crate) lock_state: u8,
  /// Platform identity captured from the freshly opened handle. See
  /// `DiskMmapFile::file_identity`.
  pub(crate) file_identity: crate::utils::FileIdentity,
}

impl DiskMmapFileMut {
  /// Returns `true` if a previous `truncate` failed in a way that left the
  /// underlying mapping detached from the backing file. A poisoned object
  /// rejects further reads, writes, flushes and truncates.
  #[inline]
  pub fn is_poisoned(&self) -> bool {
    self.poisoned
  }

  fn poison_err() -> Error {
    Error::other("mmap file was poisoned by a previous failed truncate")
  }

  /// Test-only: force the `poisoned` flag so coverage of the
  /// poison-rejection paths in `freeze`/`freeze_exec` doesn't depend on
  /// triggering an actual mid-truncate I/O failure.
  #[cfg(test)]
  #[allow(dead_code)]
  pub(crate) fn force_poison_for_test(&mut self) {
    self.poisoned = true;
  }

  /// In-place close-with-truncate so the wrapper can keep `Disk(...)` installed
  /// until every fallible step succeeds. Drops the live mapping (replacing it
  /// with an anonymous placeholder) before `set_len`; on any subsequent error
  /// the disk is marked `poisoned` but still owns its `path`/`file`, so the
  /// caller can retry `remove`/`drop_remove` or inspect the path.
  pub(crate) fn close_with_truncate_in_place(&mut self, max_sz: u64) -> Result<(), Error> {
    if self.poisoned {
      return Err(Self::poison_err());
    }
    self.flush()?;
    let placeholder = MmapOptions::new().len(1).map_anon()?;
    drop(std::mem::replace(&mut self.mmap, placeholder));

    let result: Result<(), Error> = (|| {
      self.file.set_len(max_sz)?;
      sync_file_and_parent(&self.file, &self.path)
    })();

    if result.is_err() {
      self.poisoned = true;
    }
    result
  }
}

impl_mmap_file_ext_for_mut!(DiskMmapFileMut);

impl MmapFileMutExt for DiskMmapFileMut {
  fn as_mut_slice(&mut self) -> &mut [u8] {
    if self.poisoned {
      &mut []
    } else {
      self.mmap.as_mut()
    }
  }

  fn is_cow(&self) -> bool {
    matches!(self.typ, MmapFileMutType::Cow)
  }

  impl_flush!();

  fn truncate(&mut self, max_sz: u64) -> Result<(), Error> {
    if self.poisoned {
      return Err(Self::poison_err());
    }
    if self.is_cow() {
      return Err(Error::new(
        ErrorKind::Unsupported,
        "cannot truncate a copy-on-write mmap file",
      ));
    }
    // The current offset must still fit inside the new file.
    if self.offset > max_sz {
      return Err(Error::new(
        ErrorKind::InvalidInput,
        "truncate would leave mapping offset past EOF",
      ));
    }

    // sync data
    let meta = self.file.metadata()?;
    if meta.len() > 0 {
      self.flush()?;
    }

    // Drop the existing mapping before set_len. Some platforms (Windows,
    // some BSDs) refuse to truncate a mapped file, and on Linux a successful
    // set_len followed by a remap failure would leave a stale oversized
    // mapping that SIGBUSes on access past the new EOF. Swapping in a tiny
    // anonymous placeholder keeps self.mmap a valid `MmapMut` even if any
    // step below fails; we then mark `self.poisoned = true` so callers
    // can't read/write that placeholder.
    let placeholder = MmapOptions::new().len(1).map_anon()?;
    drop(std::mem::replace(&mut self.mmap, placeholder));

    // From this point on, any failure poisons self.
    let result = (|| -> Result<MmapMut, Error> {
      self.file.set_len(max_sz)?;
      sync_file_and_parent(&self.file, &self.path)?;

      // Build a fresh MmapOptions clamping `len` to (max_sz - offset) so
      // the new mapping doesn't extend past EOF. If user didn't set an
      // explicit len, leave it unset (memmapix maps to EOF).
      let mut opts = self.opts.clone().unwrap_or_default();
      if let Some(user_len) = self.len {
        let cap = (max_sz - self.offset) as usize;
        opts.len(user_len.min(cap));
      }
      self.opts = Some(opts.clone());
      remmap(&self.file, Some(&opts), self.typ)
    })();
    match result {
      Ok(new_mmap) => {
        self.mmap = new_mmap;
        Ok(())
      }
      Err(e) => {
        self.poisoned = true;
        Err(e)
      }
    }
  }

  /// Remove the underlying file
  ///
  /// # Examples
  ///
  /// ```ignore
  /// use fmmap::MmapFileMutExt;
  /// use fmmap::raw::DiskMmapFileMut;
  ///
  /// let mut file = DiskMmapFileMut::create("disk_remove_test.txt").unwrap();
  ///
  /// file.truncate(100);
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  ///
  /// file.remove().unwrap();
  ///
  /// let err = std::fs::File::open("disk_remove_test.txt");
  /// assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
  /// ```
  fn drop_remove(self) -> crate::error::Result<()> {
    let path = self.path;
    let identity = self.file_identity;
    drop(self.mmap);
    drop(self.file);
    // Pre-open the parent dir handle so the post-unlink fsync commits
    // metadata on the *original* parent inode even if the path's parent
    // is renamed mid-operation.
    let parent_handle = crate::utils::open_parent_for_sync(&path)?;
    // Path-reuse safety: between dropping the handle and now, another
    // actor could have replaced the file at this path. Verify identity
    // before unlinking, distinguishing "path missing" from "path-reuse
    // mismatch": the former is the same NotFound semantics callers had
    // before identity tracking; the latter must refuse the delete.
    match std::fs::metadata(&path) {
      Err(e) if e.kind() == ErrorKind::NotFound => return Err(e),
      Err(e) => return Err(e),
      Ok(probe) => {
        let probe_id = crate::utils::FileIdentity::from_metadata(&probe);
        if !identity.is_known_equal(&probe_id) {
          return Err(Error::other(format!(
            "cannot unlink '{}': path no longer names the original file (path-reuse detected between handle drop and unlink, or platform identity unavailable)",
            path.display(),
          )));
        }
      }
    }
    // Initial-call semantics: a missing file is the user's error; we do
    // NOT treat NotFound as success here. (Idempotency for the
    // post-failure-pre-sync window lives at the wrapper layer where
    // `pending_drop_remove` is consulted.)
    remove_file(&path)?;
    // Sync the original parent inode handle. Tag a parent-sync failure
    // so the caller using the raw API can tell unlink-failed from
    // unlink-succeeded-but-parent-fsync-failed.
    crate::utils::sync_parent_handle(&parent_handle).map_err(|e| {
      Error::new(
        e.kind(),
        format!(
          "file '{}' unlinked but parent dir fsync failed: {e}; \
           the unlink is committed but not yet crash-durable",
          path.display(),
        ),
      )
    })
  }

  /// Close and truncate the underlying file
  ///
  /// # Examples
  ///
  /// ```ignore
  /// use fmmap::{MetaDataExt, MmapFileExt, MmapFileMutExt};
  /// use fmmap::raw::DiskMmapFileMut;
  /// # use scopeguard::defer;
  ///
  /// let mut file = DiskMmapFileMut::create("disk_close_with_truncate_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_close_with_truncate_test.txt").unwrap());
  /// file.truncate(100);
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  ///
  /// file.close_with_truncate(50).unwrap();
  ///
  /// let file = DiskMmapFileMut::open("disk_close_with_truncate_test.txt").unwrap();
  /// let meta = file.metadata().unwrap();
  /// assert_eq!(meta.len(), 50);
  /// ```
  fn close_with_truncate(self, max_sz: i64) -> crate::error::Result<()> {
    // COW mappings are private — by contract they must not mutate the
    // backing file. Refuse close-time truncation on COW so it stays in line
    // with `truncate()`.
    if max_sz >= 0 && self.is_cow() {
      return Err(Error::new(
        ErrorKind::Unsupported,
        "cannot truncate a copy-on-write mmap file",
      ));
    }
    let meta = self.file.metadata()?;
    if meta.len() > 0 {
      self.flush()?;
    }

    drop(self.mmap);
    if max_sz >= 0 {
      self.file.set_len(max_sz as u64)?;
      sync_file_and_parent(&self.file, &self.path)?;
    }
    Ok(())
  }
}

impl DiskMmapFileMut {
  /// Create a new file and mmap this file
  ///
  /// # Notes
  /// The new file is zero size, so before do write, you should truncate first.
  /// Or you can use [`create_with_options`] and set `max_size` field for [`Options`] to enable directly write
  /// without truncating.
  ///
  /// # Examples
  ///
  /// ```ignore
  /// use fmmap::MmapFileMutExt;
  /// use fmmap::raw::DiskMmapFileMut;
  /// # use scopeguard::defer;
  ///
  /// let mut file = DiskMmapFileMut::create("disk_create_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_create_test.txt").unwrap());
  /// file.truncate(100);
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  /// ```
  ///
  /// [`create_with_options`]: struct.DiskMmapFileMut.html#method.create_with_options
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn create<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
    Self::create_in(path, None)
  }

  /// Create a new file and mmap this file with [`Options`]
  ///
  /// # Examples
  ///
  /// ```ignore
  /// use fmmap::{Options, MmapFileMutExt};
  /// use fmmap::raw::DiskMmapFileMut;
  /// # use scopeguard::defer;
  ///
  /// let opts = Options::new()
  ///     // truncate to 100
  ///     .max_size(100);
  /// let mut file = DiskMmapFileMut::create_with_options("disk_create_with_options_test.txt", opts).unwrap();
  /// # defer!(std::fs::remove_file("disk_create_with_options_test.txt").unwrap());
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  /// ```
  ///
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn create_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
    Self::create_in(path, Some(opts))
  }

  /// Open or Create(if not exists) a file and mmap this file.
  ///
  /// # Notes
  /// If the file does not exist, then the new file will be open in zero size, so before do write, you should truncate first.
  /// Or you can use [`open_with_options`] and set `max_size` field for [`Options`] to enable directly write
  /// without truncating.
  ///
  /// # Examples
  ///
  /// File already exists
  ///
  /// ```ignore
  /// use fmmap::{MmapFileExt, MmapFileMutExt};
  /// use fmmap::raw::DiskMmapFileMut;
  /// use std::fs::File;
  /// use std::io::{Read, Write};
  /// # use scopeguard::defer;
  ///
  /// # let mut file = File::create("disk_open_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_open_test.txt").unwrap());
  /// # file.write_all("some data...".as_bytes()).unwrap();
  /// # drop(file);
  ///
  /// // open and mmap the file
  /// let mut file = DiskMmapFileMut::open("disk_open_test.txt").unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  ///
  /// // modify the file data
  /// file.truncate("some modified data...".len() as u64).unwrap();
  /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  /// drop(file);
  ///
  /// // reopen to check content
  /// let mut buf = vec![0; "some modified data...".len()];
  /// let mut file = File::open("disk_open_test.txt").unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
  /// ```
  ///
  /// File does not exists
  ///
  /// ```ignore
  /// use fmmap::{MmapFileExt, MmapFileMutExt};
  /// use fmmap::raw::DiskMmapFileMut;
  /// use std::fs::File;
  /// use std::io::{Read, Write};
  /// # use scopeguard::defer;
  ///
  /// // create and mmap the file
  /// let mut file = DiskMmapFileMut::open("disk_open_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_open_test.txt").unwrap());
  /// file.truncate(100).unwrap();
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  ///
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  ///
  /// // modify the file data
  /// file.truncate("some modified data...".len() as u64).unwrap();
  /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  /// drop(file);
  ///
  /// // reopen to check content
  /// let mut buf = vec![0; "some modified data...".len()];
  /// let mut file = File::open("disk_open_test.txt").unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
  /// ```
  ///
  /// [`open_with_options`]: struct.DiskMmapFileMut.html#method.open_with_options
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
    Self::open_in(path, None)
  }

  /// Open or Create(if not exists) a file and mmap this file with [`Options`].
  ///
  /// # Examples
  ///
  /// File already exists
  ///
  /// ```ignore
  /// use fmmap::{MmapFileExt, MmapFileMutExt, Options};
  /// use fmmap::raw::DiskMmapFileMut;
  /// use std::fs::File;
  /// use std::io::{Read, Seek, SeekFrom, Write};
  /// # use scopeguard::defer;
  ///
  /// # let mut file = File::create("disk_open_test_with_options.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_open_test_with_options.txt").unwrap());
  /// # file.write_all("sanity text".as_bytes()).unwrap();
  /// # file.write_all("some data...".as_bytes()).unwrap();
  /// # drop(file);
  ///
  /// // mmap the file with options
  /// let opts = Options::new()
  ///     // allow read
  ///     .read(true)
  ///     // allow write
  ///     .write(true)
  ///     // allow append
  ///     .append(true)
  ///     // truncate to 100
  ///     .max_size(100)
  ///     // mmap content after the sanity text
  ///     .offset("sanity text".as_bytes().len() as u64);
  /// let mut file = DiskMmapFileMut::open_with_options("disk_open_test_with_options.txt", opts).unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  ///
  /// // modify the file data
  /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).unwrap();
  /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  /// drop(file);
  ///
  /// // reopen to check content
  /// let mut buf = vec![0; "some modified data...".len()];
  /// let mut file = File::open("disk_open_test_with_options.txt").unwrap();
  /// file.seek(SeekFrom::Start("sanity text".as_bytes().len() as u64)).unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
  /// ```
  ///
  /// File does not exists
  ///
  /// ```ignore
  /// use fmmap::{MmapFileExt, MmapFileMutExt, Options};
  /// use fmmap::raw::DiskMmapFileMut;
  /// use std::fs::File;
  /// use std::io::{Read, Write};
  /// # use scopeguard::defer;
  ///
  /// // mmap the file with options
  /// let opts = Options::new()
  ///     // allow read
  ///     .read(true)
  ///     // allow write
  ///     .write(true)
  ///     // allow append
  ///     .append(true)
  ///     // truncate to 100
  ///     .max_size(100);
  ///
  /// let mut file = DiskMmapFileMut::open_with_options("disk_open_test_with_options.txt", opts).unwrap();
  /// # defer!(std::fs::remove_file("disk_open_test_with_options.txt").unwrap());
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  ///
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  ///
  /// // modify the file data
  /// file.truncate("some modified data...".len() as u64).unwrap();
  /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  /// drop(file);
  ///
  /// // reopen to check content
  /// let mut buf = vec![0; "some modified data...".len()];
  /// let mut file = File::open("disk_open_test_with_options.txt").unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
  /// ```
  ///
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
    Self::open_in(path, Some(opts))
  }

  /// Open an existing file and mmap this file
  ///
  /// # Examples
  /// ```ignore
  /// use fmmap::{MmapFileExt, MmapFileMutExt};
  /// use fmmap::raw::DiskMmapFileMut;
  /// use std::fs::File;
  /// use std::io::{Read, Write};
  /// # use scopeguard::defer;
  ///
  /// // create a temp file
  /// let mut file = File::create("disk_open_existing_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_open_existing_test.txt").unwrap());
  /// file.write_all("some data...".as_bytes()).unwrap();
  /// drop(file);
  ///
  /// // mmap the file
  /// let mut file = DiskMmapFileMut::open_exist("disk_open_existing_test.txt").unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  ///
  /// // modify the file data
  /// file.truncate("some modified data...".len() as u64).unwrap();
  /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  /// drop(file);
  ///
  /// // reopen to check content
  /// let mut buf = vec![0; "some modified data...".len()];
  /// let mut file = File::open("disk_open_existing_test.txt").unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
  /// ```
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
    Self::open_exist_in(path, None)
  }

  /// Open an existing file and mmap this file with [`Options`]
  ///
  /// # Examples
  /// ```ignore
  /// use fmmap::{MmapFileExt, MmapFileMutExt, Options};
  /// use fmmap::raw::DiskMmapFileMut;
  /// use std::fs::File;
  /// use std::io::{Read, Seek, SeekFrom, Write};
  /// # use scopeguard::defer;
  ///
  /// // create a temp file
  /// let mut file = File::create("disk_open_existing_test_with_options.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_open_existing_test_with_options.txt").unwrap());
  /// file.write_all("sanity text".as_bytes()).unwrap();
  /// file.write_all("some data...".as_bytes()).unwrap();
  /// drop(file);
  ///
  /// // mmap the file with options
  /// let opts = Options::new()
  ///     // truncate to 100
  ///     .max_size(100)
  ///     // mmap content after the sanity text
  ///     .offset("sanity text".as_bytes().len() as u64);
  /// let mut file = DiskMmapFileMut::open_exist_with_options("disk_open_existing_test_with_options.txt", opts).unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  ///
  /// // modify the file data
  /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).unwrap();
  /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  /// drop(file);
  ///
  /// // reopen to check content
  /// let mut buf = vec![0; "some modified data...".len()];
  /// let mut file = File::open("disk_open_existing_test_with_options.txt").unwrap();
  /// file.seek(SeekFrom::Start("sanity text".as_bytes().len() as u64)).unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
  /// ```
  ///
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open_exist_with_options<P: AsRef<Path>>(
    path: P,
    opts: Options,
  ) -> Result<Self, Error> {
    Self::open_exist_in(path, Some(opts))
  }

  /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file).
  /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
  ///
  /// # Examples
  ///
  /// ```ignore
  /// use fmmap::{MmapFileExt, MmapFileMutExt};
  /// use fmmap::raw::DiskMmapFileMut;
  /// use std::fs::File;
  /// use std::io::{Read, Write};
  /// # use scopeguard::defer;
  ///
  /// // create a temp file
  /// let mut file = File::create("disk_open_cow_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_open_cow_test.txt").unwrap());
  /// file.write_all("some data...".as_bytes()).unwrap();
  /// drop(file);
  ///
  /// // mmap the file
  /// let mut file = DiskMmapFileMut::open_cow("disk_open_cow_test.txt").unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  ///
  /// // modify the file data
  /// file.write_all("some data!!!".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  ///
  /// // cow, change will only be seen in current caller
  /// assert_eq!(file.as_slice(), "some data!!!".as_bytes());
  /// drop(file);
  ///
  /// // reopen to check content, cow will not change the content.
  /// let mut file = File::open("disk_open_cow_test.txt").unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  /// ```
  ///
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
    Self::open_cow_in(path, None)
  }

  /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file) with [`Options`].
  /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
  ///
  /// # Examples
  ///
  /// ```ignore
  /// use fmmap::{MmapFileExt, MmapFileMutExt, Options};
  /// use fmmap::raw::DiskMmapFileMut;
  /// use std::fs::File;
  /// use std::io::{Read, Seek, Write, SeekFrom};
  /// # use scopeguard::defer;
  ///
  /// // create a temp file
  /// let mut file = File::create("disk_open_cow_with_options_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_open_cow_with_options_test.txt").unwrap());
  /// file.write_all("sanity text".as_bytes()).unwrap();
  /// file.write_all("some data...".as_bytes()).unwrap();
  /// drop(file);
  ///
  /// // mmap the file with options
  /// let opts = Options::new()
  ///     // mmap content after the sanity text
  ///     .offset("sanity text".as_bytes().len() as u64);
  /// let mut file = DiskMmapFileMut::open_cow_with_options("disk_open_cow_with_options_test.txt", opts).unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  ///
  /// // modify the file data
  /// file.write_all("some data!!!".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  ///
  /// // cow, change will only be seen in current caller
  /// assert_eq!(file.as_slice(), "some data!!!".as_bytes());
  /// drop(file);
  ///
  /// // reopen to check content, cow will not change the content.
  /// let mut file = File::open("disk_open_cow_with_options_test.txt").unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// // skip the sanity text
  /// file.seek(SeekFrom::Start("sanity text".as_bytes().len() as u64)).unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  /// ```
  ///
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open_cow_with_options<P: AsRef<Path>>(
    path: P,
    opts: Options,
  ) -> Result<Self, Error> {
    Self::open_cow_in(path, Some(opts))
  }

  /// Returns an immutable version of this memory mapped buffer.
  /// If the memory map is file-backed, the file must have been opened with read permissions.
  ///
  /// # Errors
  /// This method returns an error when the underlying system call fails,
  /// which can happen for a variety of reasons,
  /// such as when the file has not been opened with read permissions.
  ///
  /// # Examples
  /// ```ignore
  /// use fmmap::MmapFileMutExt;
  /// use fmmap::raw::DiskMmapFileMut;
  /// # use scopeguard::defer;
  ///
  /// let mut file = DiskMmapFileMut::create("disk_mmap_file_freeze_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_mmap_file_freeze_test.txt").unwrap());
  /// file.truncate(12);
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  ///
  /// file.freeze().unwrap();
  /// ```
  pub fn freeze(self) -> Result<DiskMmapFile, Error> {
    if self.poisoned {
      // After a poisoned truncate `self.mmap` is the anonymous placeholder,
      // not the file's bytes. Freezing would yield a read-only `DiskMmapFile`
      // whose `as_slice()` returns the anon bytes while `path()`/`metadata()`
      // refer to the real file — silently corrupt views.
      return Err(Self::poison_err());
    }
    // Preserve the lock state we already hold — `freeze` only changes
    // mapping permissions; the OS-level file lock is unchanged.
    Ok(DiskMmapFile {
      mmap: self.mmap.make_read_only()?,
      file: self.file,
      path: self.path,
      exec: false,
      lock_state: self.lock_state,
    })
  }

  /// Transition the memory map to be readable and executable.
  /// If the memory map is file-backed, the file must have been opened with execute permissions.
  ///
  /// # Errors
  /// This method returns an error when the underlying system call fails,
  /// which can happen for a variety of reasons,
  /// such as when the file has not been opened with execute permissions
  ///
  /// # Examples
  /// ```ignore
  /// use fmmap::MmapFileMutExt;
  /// use fmmap::raw::DiskMmapFileMut;
  /// # use scopeguard::defer;
  ///
  /// let mut file = DiskMmapFileMut::create("disk_mmap_file_freeze_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("disk_mmap_file_freeze_test.txt").unwrap());
  /// file.truncate(12);
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  ///
  /// file.freeze_exec().unwrap();
  /// ```
  pub fn freeze_exec(self) -> Result<DiskMmapFile, Error> {
    if self.poisoned {
      return Err(Self::poison_err());
    }
    Ok(DiskMmapFile {
      mmap: self.mmap.make_exec()?,
      file: self.file,
      path: self.path,
      exec: true,
      lock_state: self.lock_state,
    })
  }

  fn create_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
    let path_ref = path.as_ref();
    let (file, opts_bk, max_size, offset, len) = match opts {
      None => (create_file(path_ref)?, None, 0u64, 0u64, None),
      Some(mut opts) => {
        let max_size = opts.max_size;
        let offset = opts.offset;
        let len = opts.len;
        // A writable mmap requires full read+write on the handle; preserve
        // the user-set mode/custom_flags/share_mode/access_mode that flow
        // through `opts.file_opts`, then force the open semantics that
        // `create` requires (create_new, no append, no truncate).
        let file = opts
          .file_opts
          .read(true)
          .write(true)
          .append(false)
          .create_new(true)
          .open(path_ref)?;
        (file, Some(opts.mmap_opts), max_size, offset, len)
      }
    };

    // Auto-acquire exclusive lock to prevent aliased mappings of the same file.
    <_ as fs4::FileExt>::try_lock(&file).map_err(Error::from)?;

    // The file was just `create_new`'d so it's empty; the only post-open
    // length comes from `max_size`. Validate against that planned length
    // *before* `set_len`, otherwise an invalid `offset`/`len` would
    // succeed at extending the brand-new file before erroring.
    crate::disk::validate_mapping_range(max_size, offset, len)?;
    if max_size > 0 {
      file.set_len(max_size)?;
      sync_file_and_parent(&file, path_ref)?;
    }

    let file_identity = crate::utils::FileIdentity::from_metadata(&file.metadata()?);
    let mmap = unsafe {
      match &opts_bk {
        None => MmapMut::map_mut(&file)?,
        Some(o) => o.clone().map_mut(&file)?,
      }
    };

    Ok(Self {
      mmap,
      file,
      path: path_ref.to_path_buf(),
      opts: opts_bk,
      offset,
      len,
      typ: MmapFileMutType::Normal,
      poisoned: false,
      lock_state: crate::disk::LOCK_EXCLUSIVE,
      file_identity,
    })
  }

  fn open_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
    let path_ref = path.as_ref();
    // Open the file *non-destructively* (no truncate). The user's truncate
    // flag is applied only after we hold the auto-lock, so a lock-contended
    // open doesn't destroy existing file content.
    let (file, opts_bk, truncate, max_size, offset, len) = match opts {
      None => (open_or_create_file(path_ref)?, None, false, 0, 0u64, None),
      Some(mut opts) => {
        let truncate = opts.truncate_flag;
        let max_size = opts.max_size;
        let offset = opts.offset;
        let len = opts.len;
        // A writable mmap requires the file handle to carry full write access
        // (`FILE_WRITE_DATA` on Windows). `append(true)` strips that bit on
        // Windows, so force it off here — we never write via the handle, only
        // through the mapping, so append is meaningless for `MmapFileMut`.
        let file = opts
          .file_opts
          .read(true)
          .write(true)
          .append(false)
          .create(true)
          .open(path_ref)?;
        (file, Some(opts.mmap_opts), truncate, max_size, offset, len)
      }
    };
    <_ as fs4::FileExt>::try_lock(&file).map_err(Error::from)?;

    // Compute the post-open length we *would* end up with after applying
    // truncate / max_size, then validate the mapping range against it
    // BEFORE running any destructive `set_len`. Otherwise an invalid
    // `offset`/`len` combined with `truncate(true)` would zero the file
    // and only then return Err — silent data loss.
    let current_len = file.metadata()?.len();
    let post_truncate_len = if truncate { 0 } else { current_len };
    let planned_len = if post_truncate_len == 0 && max_size > 0 {
      max_size
    } else {
      post_truncate_len
    };
    crate::disk::validate_mapping_range(planned_len, offset, len)?;

    if truncate {
      file.set_len(0)?;
      // The set_len-0 metadata change isn't crash-durable until file +
      // parent are fsynced. Without this, `Options::truncate(true)` could
      // return Ok but a crash would let the previous file contents
      // resurrect.
      sync_file_and_parent(&file, path_ref)?;
    }
    let meta = file.metadata()?;
    if meta.len() == 0 && max_size > 0 {
      file.set_len(max_size)?;
      sync_file_and_parent(&file, path_ref)?;
    }

    let file_identity = crate::utils::FileIdentity::from_metadata(&file.metadata()?);
    let mmap = unsafe {
      match &opts_bk {
        None => MmapMut::map_mut(&file)?,
        Some(o) => o.clone().map_mut(&file)?,
      }
    };
    Ok(Self {
      mmap,
      file,
      path: path_ref.to_path_buf(),
      opts: opts_bk,
      offset,
      len,
      typ: MmapFileMutType::Normal,
      poisoned: false,
      lock_state: crate::disk::LOCK_EXCLUSIVE,
      file_identity,
    })
  }

  fn open_exist_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
    let path_ref = path.as_ref();
    let file = open_exist_file(path_ref)?;
    <_ as fs4::FileExt>::try_lock(&file).map_err(Error::from)?;

    let (mmap, opts_bk, offset, len) = match opts {
      None => {
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        (mmap, None, 0u64, None)
      }
      Some(opts) => {
        let opts_bk = opts.mmap_opts.clone();
        let offset = opts.offset;
        let len = opts.len;
        // Validate against the planned post-extension length before any
        // `set_len`, so an invalid `offset`/`len` doesn't cause us to
        // grow an existing 0-byte file and only then error.
        let current_len = file.metadata()?.len();
        let planned_len = if current_len == 0 && opts.max_size > 0 {
          opts.max_size
        } else {
          current_len
        };
        crate::disk::validate_mapping_range(planned_len, offset, len)?;
        if current_len == 0 && opts.max_size > 0 {
          file.set_len(opts.max_size)?;
          sync_file_and_parent(&file, path_ref)?;
        }
        let mmap = unsafe { opts.mmap_opts.map_mut(&file)? };
        (mmap, Some(opts_bk), offset, len)
      }
    };

    let file_identity = crate::utils::FileIdentity::from_metadata(&file.metadata()?);
    Ok(Self {
      mmap,
      file,
      path: path_ref.to_path_buf(),
      opts: opts_bk,
      offset,
      len,
      typ: MmapFileMutType::Normal,
      poisoned: false,
      lock_state: crate::disk::LOCK_EXCLUSIVE,
      file_identity,
    })
  }

  fn open_cow_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
    let path_ref = path.as_ref();
    let file = open_exist_file(path_ref)?;
    // COW maps don't write to disk; a shared lock is sufficient.
    <_ as fs4::FileExt>::try_lock_shared(&file).map_err(Error::from)?;

    let (mmap, opts_bk, offset, len) = match opts {
      None => {
        let mmap = unsafe { MmapOptions::new().map_copy(&file)? };
        (mmap, None, 0u64, None)
      }
      Some(opts) => {
        let opts_bk = opts.mmap_opts.clone();
        let offset = opts.offset;
        let len = opts.len;
        crate::disk::validate_mapping_range(file.metadata()?.len(), offset, len)?;
        let mmap = unsafe { opts.mmap_opts.map_copy(&file)? };
        (mmap, Some(opts_bk), offset, len)
      }
    };

    let file_identity = crate::utils::FileIdentity::from_metadata(&file.metadata()?);
    Ok(Self {
      mmap,
      file,
      path: path_ref.to_path_buf(),
      opts: opts_bk,
      offset,
      len,
      typ: MmapFileMutType::Cow,
      poisoned: false,
      lock_state: crate::disk::LOCK_SHARED,
      file_identity,
    })
  }
}

impl_sync_tests!("disk", DiskMmapFile, DiskMmapFileMut);

#[test]
fn test_close_with_truncate_on_empty_file() {
  let file = unsafe { DiskMmapFileMut::create("disk_close_with_truncate_test.txt") }.unwrap();
  scopeguard::defer!(let _ = std::fs::remove_file("disk_close_with_truncate_test.txt"););
  file.close_with_truncate(10).unwrap();
  assert_eq!(
    10,
    File::open("disk_close_with_truncate_test.txt")
      .unwrap()
      .metadata()
      .unwrap()
      .len()
  );
}
