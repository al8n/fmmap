// `MmapFileMutType` and `remmap` are only consumed by the feature-gated
// `sync`/`tokio`/`smol` impls. With no features enabled the lib still has
// to compile cleanly under `-D warnings`, so silence dead_code here.
#[allow(dead_code)]
#[derive(Copy, Clone)]
enum MmapFileMutType {
  Cow,
  Normal,
}

// Lock-state tracking for the disk types. Constructors auto-acquire a lock;
// the public lock methods short-circuit when the desired state is already
// held, which prevents the Windows `LockFileEx`-on-already-held-handle
// deadlock that POSIX `flock` papers over via idempotent same-handle locks.
#[allow(dead_code)]
pub(crate) const LOCK_UNLOCKED: u8 = 0;
#[allow(dead_code)]
pub(crate) const LOCK_SHARED: u8 = 1;
#[allow(dead_code)]
pub(crate) const LOCK_EXCLUSIVE: u8 = 2;

// Reject mapping ranges that exceed the backing file *or* the platform's
// slice-length limit, before handing them to memmapix. memmapix accepts
// a user-configured `len` without checking against either constraint:
// length past EOF means SIGBUS on Linux/macOS, length past `isize::MAX`
// means memmapix's `validate_len` returns Err *after* the caller has
// already destructively `set_len`'d the file. We do all checks here,
// pre-destructive, so an invalid configuration never mutates the file.
//
// 32-bit reachability: a file > 2 GiB is valid on 32-bit systems but a
// slice into it isn't; users passing `max_size > isize::MAX` (or `len`
// implicitly resolving to the file length on a large file) would zero
// or extend the file before failing. The `effective_len` check below
// rejects that combination up front.
#[allow(dead_code)]
#[inline]
fn validate_mapping_range(
  file_len: u64,
  offset: u64,
  len: Option<usize>,
) -> Result<(), ::std::io::Error> {
  if offset > file_len {
    return Err(::std::io::Error::new(
      ::std::io::ErrorKind::InvalidInput,
      format!("Options::offset ({offset}) exceeds file length ({file_len})"),
    ));
  }
  // Effective mapped length: explicit `len`, else "to end of file".
  let effective_len: u64 = match len {
    Some(n) => n as u64,
    None => file_len - offset, // safe: offset <= file_len already.
  };
  if let Some(n) = len {
    let end = offset.checked_add(n as u64).ok_or_else(|| {
      ::std::io::Error::new(
        ::std::io::ErrorKind::InvalidInput,
        "Options::offset + Options::len overflows u64",
      )
    })?;
    if end > file_len {
      return Err(::std::io::Error::new(
        ::std::io::ErrorKind::InvalidInput,
        format!("Options::offset ({offset}) + Options::len ({n}) exceeds file length ({file_len})"),
      ));
    }
  }
  // memmapix's `validate_len` rejects values that don't fit `isize`.
  // Catch it here, before any caller-side `set_len` runs, so an invalid
  // configuration never zeroes/extends the file.
  if effective_len > isize::MAX as u64 {
    return Err(::std::io::Error::new(
      ::std::io::ErrorKind::InvalidInput,
      format!(
        "effective mapping length ({effective_len}) exceeds isize::MAX ({}); a Rust slice cannot represent it",
        isize::MAX,
      ),
    ));
  }
  Ok(())
}

#[allow(dead_code)]
#[inline]
fn remmap<T: ::memmapix::MmapAsRawDesc>(
  file: T,
  opts: Option<&::memmapix::MmapOptions>,
  typ: MmapFileMutType,
) -> Result<::memmapix::MmapMut, ::std::io::Error> {
  unsafe {
    match opts {
      None => match typ {
        MmapFileMutType::Cow => ::memmapix::MmapOptions::new().map_copy(file),
        MmapFileMutType::Normal => ::memmapix::MmapMut::map_mut(file),
      },
      Some(opts) => {
        let opts = opts.clone();
        match typ {
          MmapFileMutType::Cow => opts.map_copy(file),
          MmapFileMutType::Normal => opts.map_mut(file),
        }
      }
    }
  }
}

macro_rules! impl_flush {
  () => {
    fn flush(&self) -> crate::error::Result<()> {
      if self.poisoned {
        return Err(Self::poison_err());
      }
      if self.is_cow() {
        return Ok(());
      }
      self.mmap.flush()
    }

    fn flush_async(&self) -> crate::error::Result<()> {
      if self.poisoned {
        return Err(Self::poison_err());
      }
      if self.is_cow() {
        return Ok(());
      }
      self.mmap.flush_async()
    }

    fn flush_range(&self, offset: usize, len: usize) -> crate::error::Result<()> {
      if self.poisoned {
        return Err(Self::poison_err());
      }
      if self.is_cow() {
        return Ok(());
      }
      self.mmap.flush_range(offset, len)
    }

    fn flush_async_range(&self, offset: usize, len: usize) -> crate::error::Result<()> {
      if self.poisoned {
        return Err(Self::poison_err());
      }
      if self.is_cow() {
        return Ok(());
      }
      self.mmap.flush_async_range(offset, len)
    }
  };
}

// $ext is the fs4 file-extension trait providing the lock/unlock methods on
// `self.file` (`fs4::FileExt` for sync, `fs4::AsyncFileExt` for async). UFCS
// is required so that on Rust 1.89+ we still call fs4's trait method instead
// of std's inherent `File::lock`, keeping our MSRV at fs4's 1.75.
macro_rules! impl_file_lock {
  ($ext: path) => {
    // The lock methods take `&mut self`, so the borrow checker forbids
    // concurrent access to the same wrapper. `lock_state` is therefore a
    // plain `u8` — no internal synchronization is needed. Users who need
    // to share a wrapper across threads wrap it in `Arc<Mutex<...>>`
    // themselves; that keeps the hot path lock-free for single-owner use
    // and avoids blocking async runtime workers.
    #[inline]
    fn lock(&mut self) -> crate::error::Result<()> {
      match self.lock_state {
        crate::disk::LOCK_EXCLUSIVE => Ok(()),
        crate::disk::LOCK_SHARED => Err(::std::io::Error::new(
          ::std::io::ErrorKind::WouldBlock,
          "shared lock currently held; call `unlock()` first to upgrade",
        )),
        _ => {
          <_ as $ext>::lock(&self.file)?;
          self.lock_state = crate::disk::LOCK_EXCLUSIVE;
          Ok(())
        }
      }
    }

    #[inline]
    unsafe fn lock_shared(&mut self) -> crate::error::Result<()> {
      match self.lock_state {
        crate::disk::LOCK_SHARED => Ok(()),
        crate::disk::LOCK_EXCLUSIVE => Err(::std::io::Error::new(
          ::std::io::ErrorKind::WouldBlock,
          "exclusive lock currently held; call `unlock()` first to downgrade",
        )),
        _ => {
          <_ as $ext>::lock_shared(&self.file)?;
          self.lock_state = crate::disk::LOCK_SHARED;
          Ok(())
        }
      }
    }

    #[inline]
    fn try_lock(&mut self) -> crate::error::Result<()> {
      match self.lock_state {
        crate::disk::LOCK_EXCLUSIVE => Ok(()),
        crate::disk::LOCK_SHARED => Err(::std::io::Error::new(
          ::std::io::ErrorKind::WouldBlock,
          "shared lock currently held; call `unlock()` first to upgrade",
        )),
        _ => {
          <_ as $ext>::try_lock(&self.file).map_err(::std::io::Error::from)?;
          self.lock_state = crate::disk::LOCK_EXCLUSIVE;
          Ok(())
        }
      }
    }

    #[inline]
    unsafe fn try_lock_shared(&mut self) -> crate::error::Result<()> {
      match self.lock_state {
        crate::disk::LOCK_SHARED => Ok(()),
        crate::disk::LOCK_EXCLUSIVE => Err(::std::io::Error::new(
          ::std::io::ErrorKind::WouldBlock,
          "exclusive lock currently held; call `unlock()` first to downgrade",
        )),
        _ => {
          <_ as $ext>::try_lock_shared(&self.file).map_err(::std::io::Error::from)?;
          self.lock_state = crate::disk::LOCK_SHARED;
          Ok(())
        }
      }
    }

    #[inline]
    unsafe fn unlock(&mut self) -> crate::error::Result<()> {
      if self.lock_state == crate::disk::LOCK_UNLOCKED {
        return Ok(());
      }
      <_ as $ext>::unlock(&self.file)?;
      self.lock_state = crate::disk::LOCK_UNLOCKED;
      Ok(())
    }
  };
}

cfg_sync! {
  macro_rules! impl_mmap_file_ext {
    ($name: ident) => {
      impl MmapFileExt for $name {
        fn len(&self) -> usize {
          self.mmap.len()
        }

        fn as_slice(&self) -> &[u8] {
          self.mmap.as_ref()
        }

        fn path(&self) -> &Path {
          self.path.as_path()
        }

        fn metadata(&self) -> crate::error::Result<MetaData> {
          self.file.metadata().map(MetaData::disk)
        }

        impl_file_lock!(::fs4::FileExt);

        /// Whether the mmap is executable.
        #[inline]
        fn is_exec(&self) -> bool {
          self.exec
        }
      }
    };
  }

  // Mutable disk types track a `poisoned` flag for failed-truncate recovery.
  // When poisoned, `len` and `as_slice` report empty and the type is treated
  // as having no readable content, so callers can't accidentally observe the
  // anonymous placeholder mapping.
  macro_rules! impl_mmap_file_ext_for_mut {
    ($name: ident) => {
      impl MmapFileExt for $name {
        fn len(&self) -> usize {
          if self.poisoned { 0 } else { self.mmap.len() }
        }

        fn as_slice(&self) -> &[u8] {
          if self.poisoned { &[] } else { self.mmap.as_ref() }
        }

        fn path(&self) -> &Path {
          self.path.as_path()
        }

        fn metadata(&self) -> crate::error::Result<MetaData> {
          self.file.metadata().map(MetaData::disk)
        }

        impl_file_lock!(::fs4::FileExt);

        /// Whether the mmap is executable.
        #[inline]
        fn is_exec(&self) -> bool {
          false
        }
      }
    };
  }

}

#[cfg(feature = "sync")]
#[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
mod sync_impl;
#[cfg(feature = "sync")]
#[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
pub use sync_impl::{DiskMmapFile, DiskMmapFileMut};

cfg_async! {
  macro_rules! impl_async_mmap_file_ext {
    ($name: ident) => {

      impl AsyncMmapFileExt for $name {
        fn len(&self) -> usize {
          self.mmap.len()
        }

        fn as_slice(&self) -> &[u8] {
          self.mmap.as_ref()
        }

        fn path(&self) -> &Path {
          self.path.as_path()
        }

        #[inline]
        async fn metadata(&self) -> crate::error::Result<MetaData> {
          self.file
            .metadata()
            .await
            .map(MetaData::disk)
        }

        /// Whether the mmap is executable.
        #[inline]
        fn is_exec(&self) -> bool {
          self.exec
        }

        impl_file_lock!(::fs4::AsyncFileExt);
      }
    };
  }

  // Mutable async disk types track a `poisoned` flag for failed-truncate
  // recovery; when poisoned, reads report empty so callers can't observe the
  // anonymous placeholder mapping.
  macro_rules! impl_async_mmap_file_ext_for_mut {
    ($name: ident) => {

      impl AsyncMmapFileExt for $name {
        fn len(&self) -> usize {
          if self.poisoned { 0 } else { self.mmap.len() }
        }

        fn as_slice(&self) -> &[u8] {
          if self.poisoned { &[] } else { self.mmap.as_ref() }
        }

        fn path(&self) -> &Path {
          self.path.as_path()
        }

        #[inline]
        async fn metadata(&self) -> crate::error::Result<MetaData> {
          self.file
            .metadata()
            .await
            .map(MetaData::disk)
        }

        /// Whether the mmap is executable.
        #[inline]
        fn is_exec(&self) -> bool {
          false
        }

        impl_file_lock!(::fs4::AsyncFileExt);
      }
    };
  }

  macro_rules! declare_and_impl_async_fmmap_file {
    ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal, $base_file: ty) => {
      /// AsyncDiskMmapFile contains an immutable mmap buffer
      /// and a read-only file.
      pub struct AsyncDiskMmapFile {
        pub(crate) mmap: Mmap,
        pub(crate) file: $base_file,
        pub(crate) path: PathBuf,
        exec: bool,
        /// Tracks the current advisory file-lock state so the public
        /// lock methods can short-circuit when the desired lock is
        /// already held. Initialized to `LOCK_SHARED` after the
        /// constructor's auto-acquire.
        pub(crate) lock_state: u8,
      }

      impl_async_mmap_file_ext!(AsyncDiskMmapFile);

      impl AsyncDiskMmapFile {
        /// Open a readable memory map backed by a file
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::AsyncMmapFileExt;")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFile;")]
        #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        #[doc = " # use scopeguard::defer;"]
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_disk_open_test.txt\").await.unwrap();")]
        #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_test.txt\").unwrap());")]
        #[doc = concat!("# file.truncate(100).await.unwrap();")]
        #[doc = concat!("# file.write_all(\"some data...\".as_bytes(), 0).unwrap();")]
        #[doc = concat!("# file.flush().unwrap();")]
        #[doc = "# drop(file);"]
        #[doc = "// mmap the file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFile::open(\"", $filename_prefix, "_disk_open_test.txt\").await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
          Self::open_in(path, None).await
        }

        /// Open a readable memory map backed by a file with [`AsyncOptions`]
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFileExt};")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFile;")]
        #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        #[doc = " # use scopeguard::defer;"]
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_disk_open_with_options_test.txt\").await.unwrap();")]
        #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_with_options_test.txt\").unwrap());")]
        #[doc = concat!("# file.truncate(23).await.unwrap();")]
        #[doc = concat!("# file.write_all(\"sanity text\".as_bytes(), 0).unwrap();")]
        #[doc = concat!("# file.write_all(\"some data...\".as_bytes(), \"sanity text\".as_bytes().len()).unwrap();")]
        #[doc = concat!("# file.flush().unwrap();")]
        #[doc = "# drop(file);"]
        ///
        #[doc = "// mmap the file"]
        #[doc = "let opts = AsyncOptions::new()"]
        #[doc = "    // mmap content after the sanity text"]
        #[doc = "   .offset(\"sanity text\".as_bytes().len() as u64);"]
        #[doc = "// mmap the file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFile::open_with_options(\"", $filename_prefix, "_disk_open_with_options_test.txt\", opts).await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        ///
        /// [AsyncOptions`]: struct.AsyncOptions.html
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
          Self::open_in(path, Some(opts)).await
        }

        /// Open a readable and executable memory map backed by a file
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::AsyncMmapFileExt;")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFile;")]
        #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        #[doc = " # use scopeguard::defer;"]
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_disk_open_exec_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_exec_test.txt\").unwrap());")]
        #[doc = concat!("# file.truncate(100).await.unwrap();")]
        #[doc = concat!("# file.write_all(\"some data...\".as_bytes(), 0).unwrap();")]
        #[doc = concat!("# file.flush().unwrap();")]
        #[doc = "# drop(file);"]
        #[doc = "// mmap the file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFile::open_exec(\"", $filename_prefix, "_disk_open_exec_test.txt\").await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_exec<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
          Self::open_exec_in(path, None).await
        }

        /// Open a readable and executable memory map backed by a file with [`AsyncOptions`].
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFileExt};")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFile;")]
        #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        #[doc = " # use scopeguard::defer;"]
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_disk_open_exec_with_options_test.txt\").await.unwrap();")]
        #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_exec_with_options_test.txt\").unwrap());")]
        #[doc = concat!("# file.truncate(23).await.unwrap();")]
        #[doc = concat!("# file.write_all(\"sanity text\".as_bytes(), 0).unwrap();")]
        #[doc = concat!("# file.write_all(\"some data...\".as_bytes(), \"sanity text\".as_bytes().len()).unwrap();")]
        #[doc = concat!("# file.flush().unwrap();")]
        #[doc = "# drop(file);"]
        ///
        #[doc = "// mmap the file"]
        #[doc = "let opts = AsyncOptions::new()"]
        #[doc = "    // mmap content after the sanity text"]
        #[doc = "   .offset(\"sanity text\".as_bytes().len() as u64);"]
        #[doc = "// mmap the file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFile::open_exec_with_options(\"", $filename_prefix, "_disk_open_exec_with_options_test.txt\", opts).await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        ///
        /// [`AsyncOptions`]: struct.AsyncOptions.html
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_exec_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
          Self::open_exec_in(path, Some(opts)).await
        }

        async fn open_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
          let path_ref = path.as_ref();
          let file = open_read_only_file_async(path_ref).await?;
          // Auto-acquire shared lock to prevent aliased writable mappings.
          ::fs4::AsyncFileExt::try_lock_shared(&file)
            .map_err(Error::from)?;

          if let Some(opts) = opts.as_ref() {
            crate::disk::validate_mapping_range(file.metadata().await?.len(), opts.offset, opts.len)?;
          }
          let mmap = match &opts {
            None => unsafe {
              Mmap::map(&file)?
            },
            Some(opts) => unsafe {
              opts.mmap_opts.map(&file)?
            },
          };
          Ok(Self {
            mmap,
            file,
            path: path_ref.to_path_buf(),
            exec: false,
            lock_state: crate::disk::LOCK_SHARED,
          })
        }

        async fn open_exec_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
          let path_ref = path.as_ref();
          let file = open_read_only_file_async(path_ref).await?;
          ::fs4::AsyncFileExt::try_lock_shared(&file)
            .map_err(Error::from)?;

          if let Some(opts) = opts.as_ref() {
            crate::disk::validate_mapping_range(file.metadata().await?.len(), opts.offset, opts.len)?;
          }
          let mmap = match &opts {
            None => unsafe {
              MmapOptions::new().map_exec(&file)?
            },
            Some(opts) => unsafe {
              opts.mmap_opts.map_exec(&file)?
            },
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
    };
  }

  macro_rules! impl_async_mmap_file_mut_ext_for_mut {
    ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal) => {

      impl AsyncMmapFileMutExt for AsyncDiskMmapFileMut {
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

        async fn truncate(&mut self, max_sz: u64) -> Result<(), Error> {
          if self.poisoned {
            return Err(Self::poison_err());
          }
          if self.is_cow() {
            return Err(Error::new(ErrorKind::Unsupported, "cannot truncate a copy-on-write mmap file"));
          }
          if self.offset > max_sz {
            return Err(Error::new(
              ErrorKind::InvalidInput,
              "truncate would leave mapping offset past EOF",
            ));
          }

          // sync data
          let meta = self.file.metadata().await?;
          if meta.len() > 0 {
            self.flush()?;
          }

          // Drop the existing mapping before set_len. Some platforms (Windows,
          // some BSDs) refuse to truncate a mapped file, and on Linux a successful
          // set_len followed by a remap failure would leave a stale oversized
          // mapping that SIGBUSes on access past the new EOF. Swapping in a tiny
          // anonymous placeholder keeps self.mmap a valid `MmapMut` if any step
          // below fails; we mark `poisoned = true` on failure so callers can't
          // see the placeholder.
          let placeholder = MmapOptions::new()
            .len(1)
            .map_anon()?;
          drop(::core::mem::replace(&mut self.mmap, placeholder));

          // From this point on, ANY error must mark self.poisoned so the
          // anonymous placeholder isn't visible as if it were the file's
          // mapping. Run the rest in a single fallible closure and poison
          // on failure.
          let outcome: Result<(), Error> = async {
            self.file.set_len(max_sz).await?;
            self.file.sync_all().await?;
            sync_parent_async(&self.path).await?;

            let mut opts = self.opts.clone().unwrap_or_default();
            if let Some(user_len) = self.len {
              let cap = (max_sz - self.offset) as usize;
              opts.len(user_len.min(cap));
            }
            self.opts = Some(opts.clone());
            self.mmap = remmap(&self.file, Some(&opts), self.typ)?;
            Ok(())
          }.await;

          if let Err(e) = outcome {
            self.poisoned = true;
            return Err(e);
          }
          Ok(())
        }

        /// Remove the underlying file.
        ///
        /// # Durability semantics
        ///
        /// On success, the unlink is committed AND the parent directory's
        /// metadata is fsynced (crash-durable). On parent-fsync failure
        /// after a successful unlink, the unlink is committed but not
        /// crash-durable; the raw API has no `NeedsParentSync` state
        /// machine, so the caller cannot retry fsync on the original
        /// parent inode. **For crash-durable identity-checked deletion,
        /// prefer the wrapper API** (`AsyncMmapFileMut::drop_remove` /
        /// `AsyncMmapFileMut::remove`) which carries the parent handle
        /// in `PendingDelete::NeedsParentSync` and retries on the same
        /// inode.
        ///
        /// On smol under fd pressure (EMFILE), the inode-pin dup may
        /// fail and the file is not deleted; `self` is consumed and the
        /// caller has no retry path. tokio uses `into_std` to extract
        /// the pin without allocating an fd, so no EMFILE failure mode
        /// applies on tokio.
        ///
        /// # Example
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::AsyncMmapFileMutExt;")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_disk_remove_test.txt\").await.unwrap();")]
        #[doc = ""]
        #[doc = "file.truncate(100).await;"]
        #[doc = "file.write_all(\"some data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = ""]
        #[doc = "file.drop_remove().await.unwrap();"]
        #[doc = ""]
        #[doc = concat!("let err = ", $path_str, "::fs::File::open(\"", $filename_prefix, "_disk_remove_test.txt\").await;")]
        #[doc = "assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);"]
        #[doc = "# })"]
        #[doc = "```"]
        async fn drop_remove(self) -> crate::error::Result<()> {
          let path = self.path;
          let identity = self.file_identity;
          // Take ownership of the inode pin via runtime-specific
          // helper. tokio uses `tokio::fs::File::into_std` (no fd
          // alloc — closes the EMFILE-during-drop_remove window). smol
          // falls back to `fcntl_dupfd_cloexec` because async-fs has
          // no into_std equivalent; on EMFILE the raw API returns Err
          // and the file is not deleted (documented limitation; the
          // wrapper's `drop_remove`/`remove` route through their own
          // recovery path that retries via Drop).
          drop(self.mmap);
          #[cfg(unix)]
          let inode_pin: Option<std::fs::File> = match extract_pin_or_err(self.file).await {
            Ok(pin) => Some(pin),
            // Discard the recovered file; the trait method's
            // `Result<()>` shape can't return `Self`.
            Err((_file_back, e)) => return Err(e),
          };
          #[cfg(not(unix))]
          let inode_pin: Option<std::fs::File> = {
            drop(self.file);
            None
          };
          // Run the entire blocking sequence on the runtime's
          // blocking-task pool, matching the wrapper-level async paths
          // so a slow filesystem can't stall the executor.
          run_blocking_io(move || -> crate::error::Result<()> {
            let _inode_pin = inode_pin;
            let parent_handle = crate::utils::open_parent_for_sync(&path)?;
            match crate::utils::identity_at_or_path(&parent_handle, &path) {
              Err(e) if e.kind() == ErrorKind::NotFound => return Err(e),
              Err(e) => return Err(e),
              Ok(probe_id) => {
                if !identity.is_known_equal(&probe_id) {
                  return Err(Error::other(format!(
                    "cannot unlink '{}': path no longer names the original file (path-reuse detected between handle drop and unlink)",
                    path.display(),
                  )));
                }
              }
            }
            crate::utils::unlink_at_or_path(&parent_handle, &path, identity)?;
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
          })
          .await
        }

        /// Close and truncate the underlying file
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = "use fmmap::MetaDataExt;"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileExt, AsyncMmapFileMutExt};")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = "# use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_disk_close_with_truncate_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_close_with_truncate_test.txt\").unwrap());")]
        #[doc = "file.truncate(100).await;"]
        #[doc = "file.write_all(\"some data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "file.close_with_truncate(50).await.unwrap();"]
        #[doc = ""]
        #[doc = concat!("let file = AsyncDiskMmapFileMut::open(\"", $filename_prefix, "_disk_close_with_truncate_test.txt\").await.unwrap();")]
        #[doc = "let meta = file.metadata().await.unwrap();"]
        #[doc = "assert_eq!(meta.len(), 50);"]
        #[doc = "# })"]
        #[doc = "```"]
        async fn close_with_truncate(self, max_sz: i64) -> crate::error::Result<()> {
          // COW mappings are private — by contract they must not mutate the
          // backing file. Refuse close-time truncation on COW so it stays in
          // line with `truncate()`.
          if max_sz >= 0 && self.is_cow() {
            return Err(Error::new(
              ErrorKind::Unsupported,
              "cannot truncate a copy-on-write mmap file",
            ));
          }
          // sync data only if there is anything mapped that came from the file
          let meta = self.file.metadata().await?;
          if meta.len() > 0 {
            self.flush()?;
          }

          let path = self.path;
          // Drop the mapping before set_len; some platforms reject set_len on a
          // mapped file, and on all platforms the mapping is no longer needed.
          drop(self.mmap);
          if max_sz >= 0 {
            self.file.set_len(max_sz as u64).await?;
            self.file.sync_all().await?;
            sync_parent_async(&path).await?;
          }
          Ok(())
        }
      }
    };
  }

  macro_rules! declare_and_impl_async_fmmap_file_mut {
    ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal, $base_file: ty, $immutable_file: ident) => {
      /// AsyncDiskMmapFileMut contains a mutable mmap buffer
      /// and a writable file.
      pub struct AsyncDiskMmapFileMut {
        pub(crate) mmap: MmapMut,
        pub(crate) file: $base_file,
        pub(crate) path: PathBuf,
        opts: Option<MmapOptions>,
        offset: u64,
        len: Option<usize>,
        typ: MmapFileMutType,
        poisoned: bool,
        /// See `AsyncDiskMmapFile::lock_state`. Initialized to
        /// `LOCK_EXCLUSIVE` (or `LOCK_SHARED` for COW mappings) after
        /// the constructor's auto-acquire.
        pub(crate) lock_state: u8,
        /// Platform identity captured from the freshly opened handle.
        /// See `AsyncDiskMmapFile::file_identity`.
        pub(crate) file_identity: crate::utils::FileIdentity,
      }

      impl AsyncDiskMmapFileMut {
        /// Returns `true` if a previous `truncate` failed in a way that left
        /// the underlying mapping detached from the backing file. A poisoned
        /// object rejects further reads, writes, flushes and truncates.
        #[inline]
        pub fn is_poisoned(&self) -> bool {
          self.poisoned
        }

        fn poison_err() -> Error {
          Error::other("mmap file was poisoned by a previous failed truncate")
        }

        /// Test-only: force the `poisoned` flag so the
        /// poison-rejection paths in `freeze`/`freeze_exec` can be
        /// covered without triggering a real mid-truncate I/O failure.
        #[cfg(test)]
        #[allow(dead_code)]
        pub(crate) fn force_poison_for_test(&mut self) {
          self.poisoned = true;
        }

        /// In-place close-with-truncate so the wrapper can keep the
        /// `Disk(...)` variant installed until every fallible step succeeds.
        /// See the sync sibling `DiskMmapFileMut::close_with_truncate_in_place`.
        pub(crate) async fn close_with_truncate_in_place(
          &mut self,
          max_sz: u64,
        ) -> Result<(), Error> {
          if self.poisoned {
            return Err(Self::poison_err());
          }
          self.flush()?;
          let placeholder = MmapOptions::new().len(1).map_anon()?;
          drop(std::mem::replace(&mut self.mmap, placeholder));

          let result: Result<(), Error> = async {
            self.file.set_len(max_sz).await?;
            self.file.sync_all().await?;
            sync_parent_async(&self.path).await
          }
          .await;

          if result.is_err() {
            self.poisoned = true;
          }
          result
        }
      }

      impl_async_mmap_file_ext_for_mut!(AsyncDiskMmapFileMut);

      impl_async_mmap_file_mut_ext_for_mut!($filename_prefix, $doc_test_runtime, $path_str);

      impl AsyncDiskMmapFileMut {
        /// Create a new file and mmap this file
        ///
        /// # Notes
        /// The new file is zero size, so, before write, you should truncate first.
        /// Or you can use [`create_with_options`] and set `max_size` field for [`AsyncOptions`] to enable directly write
        /// without truncating.
        ///
        /// # Example
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::AsyncMmapFileMutExt;")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = " # use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_disk_create_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_create_test.txt\").unwrap());")]
        #[doc = "file.truncate(100).await;"]
        #[doc = "file.write_all(\"some data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = concat!("[`create_with_options`]: raw/", $path_str, "/struct.AsyncDiskMmapFileMut.html#method.create_with_options")]
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn create<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
          Self::create_in(path, None).await
        }

        /// Create a new file and mmap this file with [`AsyncOptions`]
        ///
        /// # Example
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFileMutExt};")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = " # use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = "let opts = AsyncOptions::new()"]
        #[doc = "     // truncate to 100"]
        #[doc = "    .max_size(100);"]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create_with_options(\"", $filename_prefix, "_disk_create_with_options_test.txt\", opts).await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_create_with_options_test.txt\").unwrap());")]
        #[doc = "file.write_all(\"some data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn create_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
          Self::create_in(path, Some(opts)).await
        }

        /// Open or Create(if not exists) a file and mmap this file.
        ///
        /// # Notes
        /// If the file does not exist, then the new file will be open in zero size, so before do write, you should truncate first.
        /// Or you can use [`open_with_options`] and set `max_size` field for [`AsyncOptions`] to enable directly write
        /// without truncating.
        ///
        /// # Examples
        ///
        /// File already exists
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileExt, AsyncMmapFileMutExt};")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = " # use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_disk_open_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_test.txt\").unwrap());")]
        #[doc = concat!("# file.truncate(100).await.unwrap();")]
        #[doc = concat!("# file.write_all(\"some data...\".as_bytes(), 0).unwrap();")]
        #[doc = concat!("# file.flush().unwrap();")]
        #[doc = "# drop(file);"]
        #[doc = ""]
        #[doc = "// mmap the file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::open(\"", $filename_prefix, "_disk_open_test.txt\").await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = ""]
        #[doc = "// modify the file data"]
        #[doc = "file.truncate(\"some modified data...\".len() as u64).await.unwrap();"]
        #[doc = "file.write_all(\"some modified data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "drop(file);"]
        #[doc = ""]
        #[doc = "// reopen to check content"]
        #[doc = "let mut buf = vec![0; \"some modified data...\".len()];"]
        #[doc = concat!("let file = AsyncDiskMmapFileMut::open(\"", $filename_prefix, "_disk_open_test.txt\").await.unwrap();")]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some modified data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = "File does not exists"]
        #[doc = ""]
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileExt, AsyncMmapFileMutExt};")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = " # use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = "// mmap the file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::open(\"", $filename_prefix, "_disk_open_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_test.txt\").unwrap());")]
        #[doc = "file.truncate(100).await.unwrap();"]
        #[doc = "file.write_all(\"some data...\".as_bytes(), 0).unwrap();"]
        #[doc = ""]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = ""]
        #[doc = "// modify the file data"]
        #[doc = "file.truncate(\"some modified data...\".len() as u64).await.unwrap();"]
        #[doc = "file.write_all(\"some modified data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "drop(file);"]
        #[doc = ""]
        #[doc = "// reopen to check content"]
        #[doc = "let mut buf = vec![0; \"some modified data...\".len()];"]
        #[doc = concat!("let file = AsyncDiskMmapFileMut::open(\"", $filename_prefix, "_disk_open_test.txt\").await.unwrap();")]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some modified data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = concat!("[`open_with_options`]: raw/", $path_str, "/struct.AsyncDiskMmapFileMut.html#method.open_with_options")]
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
          Self::open_in(path, None).await
        }

        /// Open or Create(if not exists) a file and mmap this file with [`AsyncOptions`].
        ///
        /// # Examples
        ///
        /// File already exists
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
        #[doc = "# use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_with_options_test.txt\").unwrap());")]
        #[doc = "# file.truncate(23).await.unwrap();"]
        #[doc = "# file.write_all(\"sanity text\".as_bytes(), 0).unwrap();"]
        #[doc = "# file.write_all(\"some data...\".as_bytes(), \"sanity text\".as_bytes().len()).unwrap();"]
        #[doc = "# file.flush().unwrap();"]
        #[doc = "# drop(file);"]
        #[doc = ""]
        #[doc = "let opts = AsyncOptions::new()"]
        #[doc = "    // allow read"]
        #[doc = "    .read(true)"]
        #[doc = "    // allow write"]
        #[doc = "    .write(true)"]
        #[doc = "    // allow append"]
        #[doc = "    .append(true)"]
        #[doc = "    // truncate to 100"]
        #[doc = "    .max_size(100)"]
        #[doc = "    // mmap content after the sanity text"]
        #[doc = "    .offset(\"sanity text\".as_bytes().len() as u64);"]
        #[doc = concat!("let mut file = AsyncMmapFileMut::open_with_options(\"", $filename_prefix, "_open_with_options_test.txt\", opts).await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = ""]
        #[doc = "// modify the file data"]
        #[doc = "file.truncate((\"some modified data...\".len() + \"sanity text\".len()) as u64).await.unwrap();"]
        #[doc = "file.write_all(\"some modified data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "drop(file);"]
        #[doc = ""]
        #[doc = "// reopen to check content"]
        #[doc = "let mut buf = vec![0; \"some modified data...\".len()];"]
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
        #[doc = "// skip the sanity text"]
        #[doc = "file.read_exact(buf.as_mut_slice(), \"sanity text\".as_bytes().len()).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some modified data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = "File does not exists"]
        #[doc = ""]
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
        #[doc = "# use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = "// mmap the file with options"]
        #[doc = "let opts = AsyncOptions::new()"]
        #[doc = "    // allow read"]
        #[doc = "    .read(true)"]
        #[doc = "    // allow write"]
        #[doc = "    .write(true)"]
        #[doc = "    // allow append"]
        #[doc = "    .append(true)"]
        #[doc = "    // truncate to 100"]
        #[doc = "    .max_size(100);"]
        #[doc = concat!("let mut file = AsyncMmapFileMut::open_with_options(\"", $filename_prefix, "_open_with_options_test.txt\", opts).await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_with_options_test.txt\").unwrap());")]
        #[doc = "file.write_all(\"some data...\".as_bytes(), 0).unwrap();"]
        #[doc = ""]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = ""]
        #[doc = "// modify the file data"]
        #[doc = "file.truncate(\"some modified data...\".len() as u64).await.unwrap();"]
        #[doc = "file.write_all(\"some modified data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "drop(file);"]
        #[doc = ""]
        #[doc = "// reopen to check content"]
        #[doc = "let mut buf = vec![0; \"some modified data...\".len()];"]
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some modified data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
          Self::open_in(path, Some(opts)).await
        }

        /// Open an existing file and mmap this file
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileExt, AsyncMmapFileMutExt};")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = " # use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = "// create a temp file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_disk_open_existing_test.txt\").await.unwrap();")]
        #[doc = "file.truncate(100).await.unwrap();"]
        #[doc = "file.write_all(\"some data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_existing_test.txt\").unwrap());")]
        #[doc = "drop(file);"]
        #[doc = ""]
        #[doc = "// mmap the file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::open_exist(\"", $filename_prefix, "_disk_open_existing_test.txt\").await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = ""]
        #[doc = "// modify the file data"]
        #[doc = "file.truncate(\"some modified data...\".len() as u64).await.unwrap();"]
        #[doc = "file.write_all(\"some modified data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "drop(file);"]
        #[doc = ""]
        #[doc = ""]
        #[doc = "// reopen to check content"]
        #[doc = "let mut buf = vec![0; \"some modified data...\".len()];"]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::open_exist(\"", $filename_prefix, "_disk_open_existing_test.txt\").await.unwrap();")]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some modified data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
          Self::open_exist_in(path, None).await
        }

        /// Open an existing file and mmap this file with [`AsyncOptions`]
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = " # use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = "// create a temp file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_disk_open_existing_test_with_options.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_existing_test_with_options.txt\").unwrap());")]
        #[doc = "file.truncate(23).await.unwrap();"]
        #[doc = "file.write_all(\"sanity text\".as_bytes(), 0).unwrap();"]
        #[doc = "file.write_all(\"some data...\".as_bytes(), \"sanity text\".as_bytes().len()).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "drop(file);"]
        #[doc = ""]
        #[doc = "// mmap the file"]
        #[doc = "let opts = AsyncOptions::new()"]
        #[doc = "     // truncate to 100"]
        #[doc = "    .max_size(100)"]
        #[doc = "    // mmap content after the sanity text"]
        #[doc = "   .offset(\"sanity text\".as_bytes().len() as u64);"]
        #[doc = ""]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::open_exist_with_options(\"", $filename_prefix, "_disk_open_existing_test_with_options.txt\", opts).await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = ""]
        #[doc = "// modify the file data"]
        #[doc = "file.truncate((\"some modified data...\".len() + \"sanity text\".len()) as u64).await.unwrap();"]
        #[doc = "file.write_all(\"some modified data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = ""]
        #[doc = ""]
        #[doc = "// reopen to check content, cow will not change the content."]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::open(\"", $filename_prefix, "_disk_open_existing_test_with_options.txt\").await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some modified data...\".len()];"]
        #[doc = "// skip the sanity text"]
        #[doc = "file.read_exact(buf.as_mut_slice(), \"sanity text\".as_bytes().len()).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some modified data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
          Self::open_exist_in(path, Some(opts)).await
        }

        /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file).
        /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileExt, AsyncMmapFileMutExt};")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = "# use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = "// create a temp file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_disk_open_cow_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_cow_test.txt\").unwrap());")]
        #[doc = "file.truncate(12).await.unwrap();"]
        #[doc = "file.write_all(\"some data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "drop(file);"]
        #[doc = ""]
        #[doc = "// mmap the file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::open_cow(\"", $filename_prefix, "_disk_open_cow_test.txt\").await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = ""]
        #[doc = "// modify the file data"]
        #[doc = "file.write_all(\"some data!!!\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = ""]
        #[doc = "// cow, change will only be seen in current caller"]
        #[doc = "assert_eq!(file.as_slice(), \"some data!!!\".as_bytes());"]
        #[doc = "drop(file);"]
        #[doc = ""]
        #[doc = "// reopen to check content, cow will not change the content."]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::open(\"", $filename_prefix, "_disk_open_cow_test.txt\").await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
          Self::open_cow_in(path, None).await
        }

        /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file) with [`AsyncOptions`].
        /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = concat!("use ", $path_str, "::fs::File;")]
        #[doc = "# use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = "// create a temp file"]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_disk_open_cow_with_options_test.txt\").await.unwrap();")]
        #[doc = concat!("#  defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_cow_with_options_test.txt\").unwrap());")]
        #[doc = "file.truncate(23).await.unwrap();"]
        #[doc = "file.write_all(\"sanity text\".as_bytes(), 0).unwrap();"]
        #[doc = "file.write_all(\"some data...\".as_bytes(), \"sanity text\".as_bytes().len()).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "drop(file);"]
        #[doc = ""]
        #[doc = "// mmap the file"]
        #[doc = "let opts = AsyncOptions::new()"]
        #[doc = "    // mmap content after the sanity text"]
        #[doc = "   .offset(\"sanity text\".as_bytes().len() as u64);"]
        #[doc = ""]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::open_cow_with_options(\"", $filename_prefix, "_disk_open_cow_with_options_test.txt\", opts).await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "file.read_exact(buf.as_mut_slice(), 0).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = ""]
        #[doc = "// modify the file data"]
        #[doc = "file.write_all(\"some data!!!\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = ""]
        #[doc = "// cow, change will only be seen in current caller"]
        #[doc = "assert_eq!(file.as_slice(), \"some data!!!\".as_bytes());"]
        #[doc = "drop(file);"]
        #[doc = ""]
        #[doc = "// reopen to check content, cow will not change the content."]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::open(\"", $filename_prefix, "_disk_open_cow_with_options_test.txt\").await.unwrap();")]
        #[doc = "let mut buf = vec![0; \"some data...\".len()];"]
        #[doc = "// skip the sanity text"]
        #[doc = "file.read_exact(buf.as_mut_slice(), \"sanity text\".as_bytes().len()).unwrap();"]
        #[doc = "assert_eq!(buf.as_slice(), \"some data...\".as_bytes());"]
        #[doc = "# })"]
        #[doc = "```"]
        #[doc = ""]
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_cow_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
          Self::open_cow_in(path, Some(opts)).await
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
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::AsyncMmapFileMutExt;")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = "# use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_disk_freeze_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_freeze_test.txt\").unwrap());")]
        #[doc = "file.truncate(100).await;"]
        #[doc = "file.write_all(\"some data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "// freeze"]
        #[doc = "file.freeze().unwrap();"]
        #[doc = "# })"]
        #[doc = "```"]
        pub fn freeze(self) -> Result<$immutable_file, Error> {
          if self.poisoned {
            // After a poisoned truncate `self.mmap` is the anonymous
            // placeholder, not the file's bytes. Freezing would yield a
            // read-only mapping whose `as_slice()` returns anon bytes
            // while `path()`/`metadata()` refer to the real file —
            // silently corrupt views.
            return Err(Self::poison_err());
          }
          // Preserve the lock state we already hold — `freeze` only
          // changes mapping permissions; the OS-level file lock is
          // unchanged.
          Ok($immutable_file {
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
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::AsyncMmapFileMutExt;")]
        #[doc = concat!("use fmmap::raw::", $path_str, "::AsyncDiskMmapFileMut;")]
        #[doc = "# use scopeguard::defer;"]
        #[doc = ""]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_disk_freeze_exec_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_freeze_exec_test.txt\").unwrap());")]
        #[doc = "file.truncate(100).await;"]
        #[doc = "file.write_all(\"some data...\".as_bytes(), 0).unwrap();"]
        #[doc = "file.flush().unwrap();"]
        #[doc = "// freeze_exec"]
        #[doc = "file.freeze_exec().unwrap();"]
        #[doc = "# })"]
        #[doc = "```"]
        pub fn freeze_exec(self) -> Result<$immutable_file, Error> {
          if self.poisoned {
            return Err(Self::poison_err());
          }
          Ok($immutable_file {
            mmap: self.mmap.make_exec()?,
            file: self.file,
            path: self.path,
            exec: true,
            lock_state: self.lock_state,
          })
        }
      }
    };
  }

  macro_rules! impl_async_fmmap_file_mut_private {
    ($name: ident) => {
      impl $name {
        async fn create_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
          let path_ref = path.as_ref();
          let (file, opts_bk, max_size, offset, len) = match opts {
            None => (create_file_async(path_ref).await?, None, 0u64, 0u64, None),
            Some(mut opts) => {
              let max_size = opts.max_size;
              let offset = opts.offset;
              let len = opts.len;
              // A writable mmap requires full read+write on the handle;
              // preserve the user-set mode/custom_flags/share_mode/access_mode
              // that flow through `opts.file_opts`, then force the open
              // semantics `create` requires (create_new, no append, no truncate).
              let file = opts
                .file_opts
                .read(true)
                .write(true)
                .append(false)
                .create_new(true)
                .open(path_ref)
                .await?;
              (file, Some(opts.mmap_opts), max_size, offset, len)
            }
          };

          ::fs4::AsyncFileExt::try_lock(&file)
            .map_err(Error::from)?;

          // The file was just `create_new`'d so it's empty; the only
          // post-open length comes from `max_size`. Validate before any
          // `set_len` so an invalid `offset`/`len` doesn't first extend
          // a brand-new file.
          crate::disk::validate_mapping_range(max_size, offset, len)?;
          if max_size > 0 {
            file.set_len(max_size).await?;
            file.sync_all().await?;
            sync_parent_async(&path).await?;
          }

          // Capture identity from the *already-open* async handle so
          // an attacker can't race a path swap between our open and the
          // identity probe. tokio::fs::File / smol::fs::File both impl
          // AsRawFd / AsRawHandle, so we read the raw fd/handle and
          // call the appropriate platform identity API on it.
          let file_identity = {
            #[cfg(unix)]
            {
              use std::os::fd::AsRawFd;
              // SAFETY: `file` is alive for the duration of this call;
              // we hold it across the unsafe call.
              unsafe { crate::utils::FileIdentity::from_raw_fd(file.as_raw_fd()) }?
            }
            #[cfg(windows)]
            {
              use std::os::windows::io::AsRawHandle;
              // SAFETY: `file` is alive for the duration of this call.
              unsafe { crate::utils::FileIdentity::from_raw_handle(file.as_raw_handle()) }?
            }
            #[cfg(not(any(unix, windows)))]
            crate::utils::FileIdentity::from_path(path_ref)?
          };
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

        async fn open_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
          let path_ref = path.as_ref();
          // Open the file *non-destructively* (no truncate). The user's
          // truncate flag is applied only after we hold the auto-lock, so a
          // lock-contended open doesn't destroy existing file content.
          let (file, opts_bk, truncate, max_size, offset, len) = match opts {
            None => (open_or_create_file_async(path_ref).await?, None, false, 0, 0u64, None),
            Some(mut opts) => {
              let truncate = opts.truncate_flag;
              let max_size = opts.max_size;
              let offset = opts.offset;
              let len = opts.len;
              // A writable mmap requires the file handle to carry full write
              // access (`FILE_WRITE_DATA` on Windows). `append(true)` strips
              // that bit on Windows, so force it off here — we never write via
              // the handle, only through the mapping, so append is meaningless
              // for `AsyncDiskMmapFileMut`.
              let file = opts
                .file_opts
                .read(true)
                .write(true)
                .append(false)
                .create(true)
                .open(path_ref)
                .await?;
              (file, Some(opts.mmap_opts), truncate, max_size, offset, len)
            }
          };
          ::fs4::AsyncFileExt::try_lock(&file)
            .map_err(Error::from)?;

          // Compute the planned post-open length and validate the
          // mapping range BEFORE any destructive `set_len`, otherwise
          // an invalid `offset`/`len` combined with `truncate(true)`
          // would zero the file before erroring — silent data loss.
          let current_len = file.metadata().await?.len();
          let post_truncate_len = if truncate { 0 } else { current_len };
          let planned_len = if post_truncate_len == 0 && max_size > 0 {
            max_size
          } else {
            post_truncate_len
          };
          crate::disk::validate_mapping_range(planned_len, offset, len)?;

          if truncate {
            file.set_len(0).await?;
            // Make the set_len-0 metadata change crash-durable; without
            // this, `AsyncOptions::truncate(true)` could return Ok but
            // a crash would let the previous file contents resurrect.
            file.sync_all().await?;
            sync_parent_async(&path).await?;
          }
          let meta = file.metadata().await?;
          if meta.len() == 0 && max_size > 0 {
            file.set_len(max_size).await?;
            file.sync_all().await?;
            sync_parent_async(&path).await?;
          }

          // Capture identity from the *already-open* async handle so
          // an attacker can't race a path swap between our open and the
          // identity probe. tokio::fs::File / smol::fs::File both impl
          // AsRawFd / AsRawHandle, so we read the raw fd/handle and
          // call the appropriate platform identity API on it.
          let file_identity = {
            #[cfg(unix)]
            {
              use std::os::fd::AsRawFd;
              // SAFETY: `file` is alive for the duration of this call;
              // we hold it across the unsafe call.
              unsafe { crate::utils::FileIdentity::from_raw_fd(file.as_raw_fd()) }?
            }
            #[cfg(windows)]
            {
              use std::os::windows::io::AsRawHandle;
              // SAFETY: `file` is alive for the duration of this call.
              unsafe { crate::utils::FileIdentity::from_raw_handle(file.as_raw_handle()) }?
            }
            #[cfg(not(any(unix, windows)))]
            crate::utils::FileIdentity::from_path(path_ref)?
          };
          let mmap = match &opts_bk {
            None => unsafe {
              MmapMut::map_mut(&file)?
            },
            Some(o) => unsafe {
              o.clone().map_mut(&file)?
            },
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

        async fn open_exist_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
          let path_ref = path.as_ref();
          let file = open_exist_file_async(path_ref).await?;
          ::fs4::AsyncFileExt::try_lock(&file)
            .map_err(Error::from)?;

          let (mmap, opts_bk, offset, len) = match opts {
            None => {
              let mmap = unsafe {
                MmapMut::map_mut(&file)?
              };
              (mmap, None, 0u64, None)
            }
            Some(opts) => {
              let opts_bk = opts.mmap_opts.clone();
              let offset = opts.offset;
              let len = opts.len;
              // Validate against the planned post-extension length before
              // any `set_len`, so an invalid range doesn't grow a 0-byte
              // file and only then error.
              let current_len = file.metadata().await?.len();
              let planned_len = if current_len == 0 && opts.max_size > 0 {
                opts.max_size
              } else {
                current_len
              };
              crate::disk::validate_mapping_range(planned_len, offset, len)?;
              if current_len == 0 && opts.max_size > 0 {
                file.set_len(opts.max_size).await?;
                file.sync_all().await?;
                sync_parent_async(&path).await?;
              }
              let mmap = unsafe {
                opts.mmap_opts.map_mut(&file)?
              };
              (mmap, Some(opts_bk), offset, len)
            }
          };
          // Capture identity from the *already-open* async handle so
          // an attacker can't race a path swap between our open and the
          // identity probe. tokio::fs::File / smol::fs::File both impl
          // AsRawFd / AsRawHandle, so we read the raw fd/handle and
          // call the appropriate platform identity API on it.
          let file_identity = {
            #[cfg(unix)]
            {
              use std::os::fd::AsRawFd;
              // SAFETY: `file` is alive for the duration of this call;
              // we hold it across the unsafe call.
              unsafe { crate::utils::FileIdentity::from_raw_fd(file.as_raw_fd()) }?
            }
            #[cfg(windows)]
            {
              use std::os::windows::io::AsRawHandle;
              // SAFETY: `file` is alive for the duration of this call.
              unsafe { crate::utils::FileIdentity::from_raw_handle(file.as_raw_handle()) }?
            }
            #[cfg(not(any(unix, windows)))]
            crate::utils::FileIdentity::from_path(path_ref)?
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

        async fn open_cow_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
          let path_ref = path.as_ref();
          let file = open_exist_file_async(path_ref).await?;
          // COW maps don't write to disk; a shared lock is sufficient.
          ::fs4::AsyncFileExt::try_lock_shared(&file)
            .map_err(Error::from)?;

          let (mmap, opts_bk, offset, len) = match opts {
            None => {
              let mmap = unsafe {
                MmapOptions::new().map_copy(&file)?
              };
              (mmap, None, 0u64, None)
            }
            Some(opts) => {
              let opts_bk = opts.mmap_opts.clone();
              let offset = opts.offset;
              let len = opts.len;
              crate::disk::validate_mapping_range(file.metadata().await?.len(), offset, len)?;
              let mmap = unsafe {
                opts.mmap_opts.map_copy(&file)?
              };
              (mmap, Some(opts_bk), offset, len)
            }
          };

          // Capture identity from the *already-open* async handle so
          // an attacker can't race a path swap between our open and the
          // identity probe. tokio::fs::File / smol::fs::File both impl
          // AsRawFd / AsRawHandle, so we read the raw fd/handle and
          // call the appropriate platform identity API on it.
          let file_identity = {
            #[cfg(unix)]
            {
              use std::os::fd::AsRawFd;
              // SAFETY: `file` is alive for the duration of this call;
              // we hold it across the unsafe call.
              unsafe { crate::utils::FileIdentity::from_raw_fd(file.as_raw_fd()) }?
            }
            #[cfg(windows)]
            {
              use std::os::windows::io::AsRawHandle;
              // SAFETY: `file` is alive for the duration of this call.
              unsafe { crate::utils::FileIdentity::from_raw_handle(file.as_raw_handle()) }?
            }
            #[cfg(not(any(unix, windows)))]
            crate::utils::FileIdentity::from_path(path_ref)?
          };
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
    };
  }
}

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub(crate) mod tokio_impl;

#[cfg(feature = "smol")]
#[cfg_attr(docsrs, doc(cfg(feature = "smol")))]
pub(crate) mod smol_impl;
