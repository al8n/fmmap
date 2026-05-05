macro_rules! read_impl {
  ($this:ident, $offset: tt, $typ:tt::$conv:tt) => {{
    const SIZE: usize = mem::size_of::<$typ>();
    let mut buf = [0u8; SIZE];
    $this
      .read_exact(&mut buf, $offset)
      .map(|_| <$typ>::$conv(buf))
  }};
}

#[allow(dead_code)]
#[inline]
fn checked_range(
  offset: usize,
  len: usize,
  upper_bound: usize,
) -> crate::error::Result<std::ops::Range<usize>> {
  match offset.checked_add(len) {
    Some(end) if end <= upper_bound => Ok(offset..end),
    _ => Err(crate::error::Error::from(
      crate::error::ErrorKind::UnexpectedEof,
    )),
  }
}

macro_rules! impl_from {
  ($outer: ident, $enum_inner: ident, [$($inner: ident), +$(,)?]) => {
    $(
    impl From<$inner> for $outer {
      fn from(file: $inner) -> Self {
        $outer{ inner: <$enum_inner>::from(file) }
      }
    }
    )*
  };
}

macro_rules! impl_from_mut {
  ($outer: ident, $enum_inner: ident, [$($inner: ident), +$(,)?]) => {
    $(
    impl From<$inner> for $outer {
      fn from(file: $inner) -> Self {
        $outer{
          inner: <$enum_inner>::from(file),
          remove_on_drop: false,
          deleted: false,
          pending_drop_remove: None,
          pending_remove_path: None,
        }
      }
    }
    )*
  };
}

/// Pending-deletion state machine. Distinguishes "we never managed to
/// unlink — retry must do both" from "we already unlinked the inode but
/// the parent fsync failed — retry must ONLY sync, NEVER call
/// `remove_file` again because the path may have been reused by another
/// process". This is the path-reuse safety knob.
#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
#[derive(Debug)]
pub(crate) enum PendingDelete {
  /// `remove_file` did not succeed (other than `NotFound`): retry must
  /// unlink and then parent-fsync. The captured `FileIdentity` was
  /// recorded *before* the original handle was dropped, so retry can
  /// re-open the path and confirm that it still names the same inode
  /// (path-reuse safety) before unlinking.
  ///
  /// On POSIX, `pin` is an `F_DUPFD_CLOEXEC`-duplicate of the original
  /// file descriptor, held alive across every retry. The kernel cannot
  /// recycle `(dev, ino)` while ANY descriptor referring to that
  /// open-file description is alive, so the identity check stays valid
  /// across as many retries as the user attempts. Without this, a
  /// retry on a tmpfs-style filesystem (which recycles inodes
  /// aggressively after the last fd closes) could match `(dev, ino)`
  /// against a totally unrelated file that happened to land at the
  /// same path with the same recycled inode number — and unlink it.
  NeedsUnlink {
    path: ::std::path::PathBuf,
    identity: crate::utils::FileIdentity,
    #[cfg(unix)]
    pin: std::fs::File,
  },
  /// Either `remove_file` succeeded but the parent fsync did not, or
  /// `remove_file` reported `NotFound` (the file was already gone before
  /// our call). Either way, the inode is presumed gone; retry MUST NOT
  /// call `remove_file` again — that could delete a new occupant of the
  /// same path. Only the parent fsync remains.
  ///
  /// `parent_handle` holds a `File` opened on the *original* parent
  /// directory inode (captured before unlink). Retry fsyncs THAT handle,
  /// not a freshly-opened one — without that, a parent-rename between
  /// unlink and retry would silently fsync the wrong inode and the
  /// wrapper would clear the pending state while the original unlink
  /// was never made crash-durable. `path` is retained for diagnostics
  /// only.
  NeedsParentSync {
    path: ::std::path::PathBuf,
    parent_handle: std::fs::File,
  },
}

#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
impl PendingDelete {
  /// Used by tests to inspect retained-on-Err state.
  #[allow(dead_code)]
  pub(crate) fn path(&self) -> &::std::path::Path {
    match self {
      Self::NeedsUnlink { path, .. } | Self::NeedsParentSync { path, .. } => path,
    }
  }
}

/// Best-effort durable cleanup for `Drop` after an explicit
/// `drop_remove()` / `remove()` failure was deferred via
/// `pending_drop_remove`.
///
/// `NeedsUnlink` carries a `FileIdentity` captured at construction time,
/// so even though the original `File` handle is gone we can still verify
/// the path still names the same inode before unlinking. If identity
/// matches: unlink, then parent fsync (on a freshly-opened parent — we
/// don't have a stashed handle on this branch). If not (path was reused):
/// leave alone.
///
/// `NeedsParentSync` carries the pre-opened parent dir handle from the
/// initial unlink; we fsync THAT handle so the durability commits to the
/// inode that contained our entry, even if the parent path has since
/// been renamed.
///
/// All errors are swallowed because `Drop` cannot return a `Result`.
#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
pub(crate) fn drop_complete_pending_delete(pending: PendingDelete) {
  match pending {
    PendingDelete::NeedsUnlink {
      path,
      identity,
      #[cfg(unix)]
      pin,
    } => {
      // Bind the probe + unlink + fsync to the same parent dir handle
      // (POSIX `fstatat` + `unlinkat` against an open parent fd).
      // Without this, a parent rename between probe and remove could
      // let us unlink an entry in a different directory and then fsync
      // the wrong parent. If we can't even open the parent, fall
      // through to a best-effort path-based parent fsync — never a
      // path-based unlink.
      //
      // `pin` (POSIX) is an alive dup of the original fd, so the
      // kernel cannot recycle `(dev, ino)` while we run the probe and
      // unlink below. Held to function exit — drop it AFTER the unlink
      // call returns.
      #[cfg(unix)]
      let _inode_pin = pin;
      let parent_handle = match crate::utils::open_parent_for_sync(&path) {
        Ok(h) => h,
        Err(_) => {
          let parent = match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => ::std::path::PathBuf::from("."),
          };
          let _ = crate::utils::sync_directory(&parent);
          return;
        }
      };
      if let Ok(probe) = crate::utils::identity_at_or_path(&parent_handle, &path) {
        if identity.is_known_equal(&probe) {
          let _ = crate::utils::unlink_at_or_path(&parent_handle, &path, identity);
        }
        // Identity mismatch → path was reused, do not unlink.
      }
      // Identity probe failure (path missing, symlink, etc.) → skip
      // unlink. Either way, fsync the parent we opened.
      let _ = crate::utils::sync_parent_handle(&parent_handle);
    }
    PendingDelete::NeedsParentSync {
      path: _,
      parent_handle,
    } => {
      let _ = crate::utils::sync_parent_handle(&parent_handle);
    }
  }
}

/// Drop-time best-effort cleanup for the opt-in `remove_on_drop` /
/// `pending_remove_path` paths.
///
/// **Does NOT call `remove_file`.** The wrapper has by this point dropped
/// the original `File` handle (via the `inner.Disk` move into Empty, or
/// via the failing `close_with_truncate` that set `pending_remove_path`),
/// so we have no way to verify the path still names the file we
/// originally opened. Calling `remove_file(path)` from `Drop` could
/// silently delete a different file that another actor created at the
/// same path, and `Drop` cannot return that error to surface the bug.
///
/// We only fsync the parent directory (idempotent and identity-free) so
/// any pre-existing pending metadata (e.g. a prior `remove`/`set_len`
/// that had succeeded but didn't yet sync) is committed.
///
/// Callers who really want auto-unlink at Drop must do their own
/// identity-checked cleanup before dropping (e.g. `file.remove()` or
/// `file.drop_remove()`), where a fresh, currently-held `File` handle is
/// available to compare against the path before unlinking.
#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
pub(crate) fn drop_unlink_durably(path: &::std::path::Path) {
  let parent = match path.parent() {
    Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
    _ => ::std::path::PathBuf::from("."),
  };
  let _ = crate::utils::sync_directory(&parent);
}

/// Identity-checked Drop-time unlink for the direct `remove_on_drop`
/// path. Uses the same parent-bound sequence as `initial_remove_durably`
/// — open the parent dir handle once, probe identity bound to it
/// (`fstatat` on POSIX), unlink bound to it (`unlinkat`), fsync the
/// same handle. `inode_pin` (POSIX only — Windows passes `None`) is
/// the original file handle, held alive through probe + unlink so the
/// `(dev, ino)` we compare against can't be recycled to a replacement.
/// All errors swallowed because `Drop` cannot return a `Result`.
#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
pub(crate) fn drop_unlink_with_identity(
  path: &::std::path::Path,
  identity: crate::utils::FileIdentity,
  inode_pin: Option<std::fs::File>,
) {
  // On POSIX, identity-checked unlink without an active pin is
  // unsafe — between the original handle's close and the identity
  // probe, the kernel can recycle `(dev, ino)` so a path-reused file
  // unrelated to ours passes the identity comparison and gets
  // unlinked. If the caller couldn't dup the fd (EMFILE etc.) and
  // passed `None`, refuse the unlink and fsync the parent only.
  #[cfg(unix)]
  let _inode_pin: std::fs::File = match inode_pin {
    Some(f) => f,
    None => {
      let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => ::std::path::PathBuf::from("."),
      };
      let _ = crate::utils::sync_directory(&parent);
      return;
    }
  };
  #[cfg(not(unix))]
  // Windows: file_index/volume_serial isn't recycled the way Unix
  // (dev, ino) is; no pin is required for path-reuse safety.
  let _inode_pin = inode_pin;
  let parent = match crate::utils::open_parent_for_sync(path) {
    Ok(p) => p,
    Err(_) => {
      // Can't even open parent — fall back to fsync-by-path of the
      // path's parent component. No unlink (we'd be path-based
      // without the parent_handle that binds the unlink to the same
      // open directory we identity-probed).
      let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => ::std::path::PathBuf::from("."),
      };
      let _ = crate::utils::sync_directory(&parent);
      return;
    }
  };
  // Probe identity bound to parent.
  let probe = match crate::utils::identity_at_or_path(&parent, path) {
    Ok(p) => p,
    Err(_) => {
      // Identity probe failed (path missing, symlink refused, etc.)
      // Skip the unlink; just sync the parent for any pending state.
      let _ = crate::utils::sync_parent_handle(&parent);
      return;
    }
  };
  if !identity.is_known_equal(&probe) {
    // Path-reuse: leave the path-reused file alone.
    let _ = crate::utils::sync_parent_handle(&parent);
    return;
  }
  // Unlink bound to the same parent handle (Unix) / same handle as
  // identity probe (Windows).
  let _ = crate::utils::unlink_at_or_path(&parent, path, identity);
  let _ = crate::utils::sync_parent_handle(&parent);
}

macro_rules! impl_drop {
  ($name: ident, $inner: ident, $empty: ident) => {
    impl Drop for $name {
      fn drop(&mut self) {
        if self.deleted {
          return;
        }
        // `pending_drop_remove` represents an explicit, user-requested
        // deletion that failed and was deferred to Drop — we MUST retry it
        // regardless of `remove_on_drop` (the user already asked for delete).
        if let Some(pending) = self.pending_drop_remove.take() {
          crate::mmap_file::drop_complete_pending_delete(pending);
          return;
        }
        // Otherwise honor the opt-in `remove_on_drop` cleanup, but ONLY for
        // Disk-backed variants. Memory variants store the user-supplied
        // string as a label, not a real on-disk path, so unlinking it
        // would delete an unrelated real file. The explicit `remove()`
        // method already no-ops for non-Disk variants; Drop must match.
        if self.remove_on_drop {
          if let Some(path) = self.pending_remove_path.take() {
            // Path-reuse safety: identity was lost when `close_with_truncate`
            // consumed the inner. We only fsync the parent now; see
            // `drop_unlink_durably` doc.
            crate::mmap_file::drop_unlink_durably(&path);
            return;
          }
          // Direct `remove_on_drop` path: the Disk inner is still alive,
          // so we can capture identity from `disk.file_identity` (recorded
          // at construction), pin the inode via the file's fd (POSIX),
          // and run the same parent-bound unlink sequence as
          // `initial_remove_durably`.
          if matches!(&self.inner, $inner::Disk(_)) {
            let empty = <$inner>::Empty(<$empty>::default());
            let inner = mem::replace(&mut self.inner, empty);
            if let $inner::Disk(disk) = inner {
              let identity = disk.file_identity;
              let path = disk.path;
              drop(disk.mmap);
              // Take the inode pin via a per-impl-file helper. Sync
              // owns the std::fs::File directly — moving it costs
              // nothing and never fails (no fd alloc). Async helpers
              // extract via `try_into_std` (tokio) or dup (smol). The
              // helper consumes disk.file, so the pin and the original
              // handle never coexist as separate fd owners.
              let pin: Option<std::fs::File> = sync_drop_pin(disk.file);
              crate::mmap_file::drop_unlink_with_identity(&path, identity, pin);
            }
          }
        }
      }
    }
  };
}

macro_rules! impl_flush {
  () => {
    fn flush(&self) -> Result<()> {
      self.inner.flush()
    }

    fn flush_async(&self) -> Result<()> {
      self.inner.flush_async()
    }

    fn flush_range(&self, offset: usize, len: usize) -> Result<()> {
      self.inner.flush_range(offset, len)
    }

    fn flush_async_range(&self, offset: usize, len: usize) -> Result<()> {
      self.inner.flush_async_range(offset, len)
    }
  };
}

macro_rules! impl_file_lock {
  () => {
    #[inline]
    fn lock(&mut self) -> crate::error::Result<()> {
      self.inner.lock()
    }

    #[inline]
    unsafe fn lock_shared(&mut self) -> crate::error::Result<()> {
      unsafe { self.inner.lock_shared() }
    }

    #[inline]
    fn try_lock(&mut self) -> crate::error::Result<()> {
      self.inner.try_lock()
    }

    #[inline]
    unsafe fn try_lock_shared(&mut self) -> crate::error::Result<()> {
      unsafe { self.inner.try_lock_shared() }
    }

    #[inline]
    unsafe fn unlock(&mut self) -> crate::error::Result<()> {
      unsafe { self.inner.unlock() }
    }
  };
}

macro_rules! impl_constructor_for_memory_mmap_file {
  ($memory_base: ident, $name: ident, $name_str: literal, $path_str: literal) => {
    use bytes::Bytes;

    impl $name {
      #[doc = concat!("Create a in-memory ", $name_str)]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = "use bytes::{BufMut, BytesMut};"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = "let mut data = BytesMut::with_capacity(100);"]
      #[doc = "data.put_slice(\"some data...\".as_bytes());"]
      #[doc = concat!($name_str, "::memory(\"foo.mem\", data.freeze());")]
      #[doc = "```"]
      pub fn memory<P: AsRef<Path>>(path: P, data: Bytes) -> Self {
        Self::from(<$memory_base>::new(path, data))
      }

      #[doc = concat!("Create a in-memory ", $name_str, " from Vec")]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = "let data = (0..=255u8).collect::<Vec<_>>();"]
      #[doc = concat!($name_str, "::memory_from_vec(\"foo.mem\", data);")]
      #[doc = "```"]
      pub fn memory_from_vec<P: AsRef<Path>>(path: P, src: Vec<u8>) -> Self {
        Self::from(<$memory_base>::from_vec(path, src))
      }

      #[doc = concat!("Create a in-memory ", $name_str, " from String")]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = "let data: &'static str = \"some data...\";"]
      #[doc = concat!($name_str, "::memory_from_string(\"foo.mem\", data.to_string());")]
      #[doc = "```"]
      pub fn memory_from_string<P: AsRef<Path>>(path: P, src: String) -> Self {
        Self::from(<$memory_base>::from_string(path, src))
      }

      #[doc = concat!("Create a in-memory ", $name_str, " from static slice")]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = "use bytes::Bytes;"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = "let data: &'static [u8] = \"some data...\".as_bytes();"]
      #[doc = concat!($name_str, "::memory_from_slice(\"foo.mem\", data);")]
      #[doc = "```"]
      pub fn memory_from_slice<P: AsRef<Path>>(path: P, src: &'static [u8]) -> Self {
        Self::from(<$memory_base>::from_slice(path, src))
      }

      #[doc = concat!("Create a in-memory ", $name_str, " from static str")]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = "use bytes::Bytes;"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = "let data: &'static str = \"some data...\";"]
      #[doc = concat!($name_str, "::memory_from_str(\"foo.mem\", data);")]
      #[doc = "```"]
      pub fn memory_from_str<P: AsRef<Path>>(path: P, src: &'static str) -> Self {
        Self::from(<$memory_base>::from_str(path, src))
      }

      #[doc = concat!("Create a in-memory ", $name_str, " by copy from slice")]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = concat!($name_str, "::memory_copy_from_slice(\"foo.mem\", \"some data...\".as_bytes());")]
      #[doc = "```"]
      pub fn memory_copy_from_slice<P: AsRef<Path>>(path: P, src: &[u8]) -> Self {
        Self::from(<$memory_base>::copy_from_slice(path, src))
      }
    }
  };
}

macro_rules! impl_constructor_for_memory_mmap_file_mut {
  ($memory_base: ident, $name: ident, $name_str: literal, $path_str: literal) => {
    impl $name {
      #[doc = concat!("Create a in-memory ", $name_str)]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = concat!($name_str, "::memory(\"foo.mem\");")]
      #[doc = "```"]
      pub fn memory<P: AsRef<Path>>(path: P) -> Self {
        Self::from(<$memory_base>::new(path))
      }

      #[doc = concat!("Create a in-memory ", $name_str, "with capacity")]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = concat!($name_str, "::memory_with_capacity(\"foo.mem\", 1000);")]
      #[doc = "```"]
      pub fn memory_with_capacity<P: AsRef<Path>>(path: P, cap: usize) -> Self {
        Self::from(<$memory_base>::with_capacity(path, cap))
      }

      #[doc = concat!("Create a in-memory ", $name_str, " from Vec")]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = "let data = (0..=255u8).collect::<Vec<_>>();"]
      #[doc = concat!($name_str, "::memory_from_vec(\"foo.mem\", data);")]
      #[doc = "```"]
      pub fn memory_from_vec<P: AsRef<Path>>(path: P, src: Vec<u8>) -> Self {
        Self::from(<$memory_base>::from_vec(path, src))
      }

      #[doc = concat!("Create a in-memory ", $name_str, " from String")]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = "let data: &'static str = \"some data...\";"]
      #[doc = concat!($name_str, "::memory_from_string(\"foo.mem\", data.to_string());")]
      #[doc = "```"]
      pub fn memory_from_string<P: AsRef<Path>>(path: P, src: String) -> Self {
        Self::from(<$memory_base>::from_string(path, src))
      }

      #[doc = concat!("Create a in-memory ", $name_str, " from static str")]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = "use bytes::Bytes;"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = "let data: &'static str = \"some data...\";"]
      #[doc = concat!($name_str, "::memory_from_str(\"foo.mem\", data);")]
      #[doc = "```"]
      pub fn memory_from_str<P: AsRef<Path>>(path: P, src: &'static str) -> Self {
        Self::from(<$memory_base>::from_str(path, src))
      }

      #[doc = concat!("Create a in-memory ", $name_str, " by from slice")]
      #[doc = "# Examples"]
      #[doc = "```ignore"]
      #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
      #[doc = ""]
      #[doc = concat!($name_str, "::memory_from_slice(\"foo.mem\", \"some data...\".as_bytes());")]
      #[doc = "```"]
      pub fn memory_from_slice<P: AsRef<Path>>(path: P, src: &[u8]) -> Self {
        Self::from(<$memory_base>::from_slice(path, src))
      }
    }
  };
}

cfg_sync! {
  macro_rules! impl_mmap_file_ext {
    ($name: ident) => {
      impl MmapFileExt for $name {
        #[inline]
        fn len(&self) -> usize {
          self.inner.len()
        }

        #[inline]
        fn as_slice(&self) -> &[u8] {
          self.inner.as_slice()
        }

        #[inline]
        fn path(&self) -> &Path {
          self.inner.path()
        }

        #[inline]
        fn is_exec(&self) -> bool {
          self.inner.is_exec()
        }

        #[inline]
        fn metadata(&self) -> Result<MetaData> {
          self.inner.metadata()
        }

        impl_file_lock!();
      }
    };
  }

}

#[cfg(feature = "sync")]
#[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
mod sync_impl;
#[cfg(feature = "sync")]
#[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
pub use sync_impl::{MmapFile, MmapFileExt, MmapFileMut, MmapFileMutExt};

cfg_async! {
  macro_rules! impl_async_mmap_file_ext {
    ($name: ident) => {

      impl AsyncMmapFileExt for $name {
        #[inline]
        fn len(&self) -> usize {
          self.inner.len()
        }

        #[inline]
        fn as_slice(&self) -> &[u8] {
          self.inner.as_slice()
        }

        #[inline]
        fn path(&self) -> &Path {
          self.inner.path()
        }

        #[inline]
        fn is_exec(&self) -> bool {
          self.inner.is_exec()
        }

        #[inline]
        async fn metadata(&self) -> Result<MetaData> {
          self.inner.metadata().await
        }

        impl_file_lock!();
      }
    };
  }

  macro_rules! impl_async_mmap_file_mut_ext {
    ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal) => {

      impl AsyncMmapFileMutExt for AsyncMmapFileMut {
        #[inline]
        fn as_mut_slice(&mut self) -> &mut [u8] {
          self.inner.as_mut_slice()
        }

        #[inline]
        fn is_cow(&self) -> bool {
          self.inner.is_cow()
        }

        impl_flush!();

        #[inline]
        async fn truncate(&mut self, max_sz: u64) -> Result<()> {
          // Just dispatch — the disk backend's `truncate` already keeps the
          // poisoned `AsyncDiskMmapFileMut` installed with its `path`/`file`
          // intact, and the disk-side accessors short-circuit to empty when
          // `poisoned == true`. Swapping the inner to `Empty` here would
          // silently lose the path so `Drop`'s `remove_on_drop` cleanup
          // (and any subsequent `remove()` / `drop_remove()` retry)
          // couldn't find the (possibly-resized) file.
          self.inner.truncate(max_sz).await
        }

        /// Remove the underlying file
        ///
        /// # Example
        ///
        /// ```ignore
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        #[doc = concat!("use ", $path_str, "::fs::File;")]
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_remove_test.txt\").await.unwrap();")]
        ///
        /// file.truncate(12).await;
        /// file.write_all("some data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        ///
        /// file.drop_remove().await.unwrap();
        ///
        #[doc = concat!("let err = File::open(\"", $filename_prefix, "_remove_test.txt\").await;")]
        /// assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
        /// # })
        /// ```
        async fn drop_remove(mut self) -> Result<()> {
          let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
          let inner = mem::replace(&mut self.inner, empty);
          match inner {
            #[allow(unused_mut)] // `mut` only needed on Unix smol-EMFILE recovery
            AsyncMmapFileMutInner::Disk(mut disk) => {
              // Run the durable unlink at the wrapper layer so we can
              // classify failures correctly (`NeedsUnlink` vs
              // `NeedsParentSync`). Delegating to the disk inner would
              // collapse parent-sync failures into a generic Err, and
              // Drop's retry could call `remove_file` on a path that's
              // already been unlinked and possibly reused.
              let path = disk.path.clone();
              let identity = disk.file_identity;
              // Take the inode pin via a runtime-specific helper.
              // tokio's `extract_pin_or_err` uses
              // `tokio::fs::File::into_std` (no fd alloc — never fails
              // for fd reasons). smol's helper still needs to dup;
              // on EMFILE the original file is returned in Err and
              // we restore self.inner for a clean Err to the caller.
              //
              // Deterministic Err semantics: we do NOT set
              // `remove_on_drop = true` here — that would trigger a
              // hidden Drop-time dup retry which, if it happens to
              // succeed, would delete the file *despite* the caller
              // having seen Err. For retryable delete, callers should
              // use the wrapper's `remove(&mut self)` API which
              // preserves `self` for an explicit retry.
              #[cfg(unix)]
              let pin: std::fs::File = {
                let async_file = disk.file;
                match extract_pin_or_err(async_file).await {
                  Ok(p) => p,
                  Err((file_back, e)) => {
                    // smol EMFILE: clean Err. File remains on disk;
                    // caller can use std::fs::remove_file or retry
                    // via remove(&mut self) once fd pressure subsides.
                    disk.file = file_back;
                    self.inner = AsyncMmapFileMutInner::Disk(disk);
                    return Err(e);
                  }
                }
              };
              #[cfg(not(unix))]
              drop(disk.file);
              // disk.file already moved into the helper on Unix Ok
              // path; mmap drops cleanly here.
              drop(disk.mmap);
              match initial_remove_durably_async(
                &path,
                identity,
                #[cfg(unix)]
                pin,
              )
              .await
              {
                Ok(()) => {
                  self.deleted = true;
                  Ok(())
                }
                Err((pending, e)) => {
                  self.pending_drop_remove = Some(pending);
                  Err(e)
                }
              }
            }
            _ => {
              // Memory/Empty drop_remove is a no-op.
              self.deleted = true;
              Ok(())
            }
          }
        }

        /// Close and truncate the underlying file
        ///
        /// # Examples
        ///
        /// ```ignore
        #[doc = concat!("use fmmap::{MetaDataExt,", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt}};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_close_with_truncate_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_close_with_truncate_test.txt\").unwrap());")]
        /// file.truncate(100).await;
        /// file.write_all("some data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        ///
        /// file.close_with_truncate(50).await.unwrap();
        ///
        #[doc = concat!("let file = AsyncMmapFileMut::open(\"", $filename_prefix, "_close_with_truncate_test.txt\").await.unwrap();")]
        /// let meta = file.metadata().await.unwrap();
        /// assert_eq!(meta.len(), 50);
        /// # })
        /// ```
        async fn close_with_truncate(mut self, max_sz: i64) -> Result<()> {
          // COW mappings are private — refuse close-time truncation
          // before touching the inner so the original mapping stays
          // usable on error.
          if max_sz >= 0 && self.is_cow() {
            return Err(Error::new(
              ErrorKind::Unsupported,
              "cannot truncate a copy-on-write mmap file",
            ));
          }

          let path = self.inner.path_buf();

          if max_sz >= 0 {
            // In-place fallible work, mirroring inherent `close()`'s
            // recovery model — partial failure leaves the disk inner
            // poisoned but intact instead of stranding the wrapper with
            // `Empty` on error.
            if let AsyncMmapFileMutInner::Disk(disk) = &mut self.inner {
              if let Err(e) = disk.close_with_truncate_in_place(max_sz as u64).await {
                if !path.as_os_str().is_empty() {
                  self.pending_remove_path = Some(path);
                }
                return Err(e);
              }
            }
          } else if let Err(e) = self.flush() {
            if !path.as_os_str().is_empty() {
              self.pending_remove_path = Some(path);
            }
            return Err(e);
          }

          let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
          drop(mem::replace(&mut self.inner, empty));
          Ok(())
        }
      }
    };
  }

  macro_rules! declare_async_mmap_file_ext {
    ($disk_file_mut: ty, $opts: ty, $reader: ty, $async_open_options: ty) => {
      /// Utility methods to [`AsyncMmapFile`]
      ///
      /// [`AsyncMmapFile`]: structs.AsyncMmapFile.html

      #[enum_dispatch]
      pub trait AsyncMmapFileExt: Sync {
        /// Returns the current mmap length
        fn len(&self) -> usize;

        /// Returns the mmap is empty of not.
        fn is_empty(&self) -> bool {
          self.len() == 0
        }

        /// Returns the underlying slice of the mmap
        fn as_slice(&self) -> &[u8];

        /// slice returns data starting from offset off of size sz.
        ///
        /// # Panics
        /// If there's not enough data, or if `offset + sz` overflows, this panics.
        fn slice(&self, offset: usize, sz: usize) -> &[u8] {
          let end = offset
            .checked_add(sz)
            .expect("offset + sz overflows usize");
          &self.as_slice()[offset..end]
        }

        /// bytes returns data starting from offset off of size sz.
        ///
        /// # Errors
        /// If there's not enough data, it would return
        /// `Err(Error::from(ErrorKind::UnexpectedEof))`.
        fn bytes(&self, offset: usize, sz: usize) -> Result<&[u8]> {
          let buf = self.as_slice();
          crate::mmap_file::checked_range(offset, sz, buf.len())
            .map(|range| &buf[range])
        }

        /// Returns the path of the inner file.
        fn path(&self) -> &Path;

        /// Returns the path buf of the inner file.
        fn path_buf(&self) -> PathBuf {
          self.path().to_path_buf()
        }

        /// Returns the path lossy string of the inner file.
        fn path_lossy(&self) -> Cow<'_, str> {
          self.path().to_string_lossy()
        }

        /// Returns the path string of the inner file.
        fn path_string(&self) -> String {
          self.path_lossy().to_string()
        }

        /// Whether the mmap is executable
        fn is_exec(&self) -> bool;

        /// Returns the metadata of file metadata
        ///
        /// Metadata information about a file.
        /// This structure is returned from the metadata or
        /// symlink_metadata function or method and represents
        /// known metadata about a file such as its permissions, size, modification times, etc
        fn metadata(&self) -> impl core::future::Future<Output = Result<MetaData>> + Send;

        /// Copy the content of the mmap file to Vec
        #[inline]
        fn copy_all_to_vec(&self) -> Vec<u8> {
          self.as_slice().to_vec()
        }

        /// Copy a range of content of the mmap file to Vec
        #[inline]
        fn copy_range_to_vec(&self, offset: usize, len: usize) -> Vec<u8> {
          self.slice(offset, len).to_vec()
        }

        /// Write the content of the mmap file to a new file.
        ///
        /// The destination is opened with plain `OpenOptions::create_new(true)`
        /// — we deliberately do NOT mmap it. Mmapping a destination would
        /// push the crate's "no concurrent mutators / truncators"
        /// precondition onto every safe caller of this helper, which is a
        /// footgun: a caller in shared storage where another actor
        /// truncates the destination during the write would hit UB / SIGBUS.
        ///
        /// On success the new file is durably created: bytes synced via
        /// `sync_all`, and the parent directory fsynced so the new
        /// directory entry is committed too.
        #[inline]
        fn write_all_to_new_file<P: AsRef<Path> + Send + Sync>(&self, new_file_path: P) -> impl core::future::Future<Output = Result<()>> + Send {
          async move {
            let path = new_file_path.as_ref();
            let buf = self.as_slice();
            let mut file = <$async_open_options>::new()
              .create_new(true)
              .read(true)
              .write(true)
              .open(path)
              .await?;
            file.write_all(buf).await?;
            file.sync_all().await?;
            let parent = match path.parent() {
              Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
              _ => ::std::path::PathBuf::from("."),
            };
            sync_dir_async(&parent).await
          }
        }

        /// Write a range of content of the mmap file to new file.
        #[inline]
        fn write_range_to_new_file<P: AsRef<Path> + Send + Sync>(&self, new_file_path: P, offset: usize, len: usize) -> impl core::future::Future<Output = Result<()>> + Send {
          async move {
            let path = new_file_path.as_ref();
            let buf = self.as_slice();
            let range = crate::mmap_file::checked_range(offset, len, buf.len())?;
            // See `write_all_to_new_file` for the no-mmap rationale.
            let mut file = <$async_open_options>::new()
              .create_new(true)
              .read(true)
              .write(true)
              .open(path)
              .await?;
            file.write_all(&buf[range]).await?;
            file.sync_all().await?;
            let parent = match path.parent() {
              Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
              _ => ::std::path::PathBuf::from("."),
            };
            sync_dir_async(&parent).await
          }
        }

        /// Returns a [`AsyncMmapFileReader`] which helps read data from mmap like a normal File.
        ///
        /// # Errors
        /// If there's not enough data, it would return
        ///  `Err(Error::from(ErrorKind::UnexpectedEof))`.
        ///
        /// [`AsyncMmapFileReader`]: structs.AsyncMmapFileReader.html
        fn reader(&self, offset: usize) -> Result<$reader> {
          let buf = self.as_slice();
          if buf.len() < offset {
            Err(Error::from(ErrorKind::UnexpectedEof))
          } else {
            Ok(<$reader>::new(Cursor::new(&buf[offset..]), offset, buf.len() - offset))
          }
        }

        /// Returns a [`AsyncMmapFileReader`] base on the given `offset` and `len`, which helps read data from mmap like a normal File.
        ///
        /// # Errors
        /// If there's not enough data, it would return
        ///  `Err(Error::from(ErrorKind::UnexpectedEof))`.
        ///
        /// [`AsyncMmapFileReader`]: structs.AsyncMmapFileReader.html
        fn range_reader(&self, offset: usize, len: usize) -> Result<$reader> {
          let buf = self.as_slice();
          let range = crate::mmap_file::checked_range(offset, len, buf.len())?;
          Ok(<$reader>::new(Cursor::new(&buf[range]), offset, len))
        }

        /// Locks the file for exclusive usage, blocking if the file is currently locked.
        ///
        /// # Notes
        /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
        fn lock(&mut self) -> Result<()>;

        /// Locks the file for shared usage, blocking if the file is currently locked exclusively.
        ///
        /// # Safety
        /// On an `AsyncMmapFileMut` the constructor auto-acquired an exclusive
        /// lock to guarantee that no other writable or read-only mapping of the
        /// same file can be opened. Calling `lock_shared` on `flock`-style
        /// platforms downgrades that exclusive lock to a shared lock, which
        /// then allows another process (or another `fmmap` handle in the same
        /// process) to open a read-only mapping of the same file. The resulting
        /// concurrent `&mut [u8]` from this writer and `&[u8]` from the reader
        /// alias the same bytes — which is undefined behavior.
        ///
        /// Callers must ensure no conflicting mapping of the same file can be
        /// created for as long as this mapping (and any borrowed slices it has
        /// yielded) lives.
        ///
        /// On a read-only `AsyncMmapFile` this call is a no-op (the auto lock
        /// is already shared) and is sound, but is still marked `unsafe`
        /// because the trait is shared between read-only and writable types.
        ///
        /// # Notes
        /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
        unsafe fn lock_shared(&mut self) -> Result<()>;

        /// Locks the file for exclusive usage, or returns a an error if the file is currently locked (see lock_contended_error).
        ///
        /// # Notes
        /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
        fn try_lock(&mut self) -> Result<()>;

        /// Locks the file for shared usage, or returns a an error if the file is currently locked exclusively (see lock_contended_error).
        ///
        /// # Safety
        /// Same hazard as [`lock_shared`]: on a writable mapping this can
        /// downgrade the auto-acquired exclusive lock to a shared lock and
        /// allow another concurrent mapping of the same file, producing
        /// aliasing UB.
        ///
        /// # Notes
        /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
        unsafe fn try_lock_shared(&mut self) -> Result<()>;

        /// Unlocks the file.
        ///
        /// # Safety
        /// `AsyncMmapFile`/`AsyncMmapFileMut` constructors automatically
        /// acquire a file lock (shared or exclusive) to prevent the underlying
        /// file from being mapped concurrently with conflicting access.
        /// Calling `unlock` releases that guard; if any other process or
        /// `fmmap` instance subsequently opens the same file with a writable
        /// mapping while this mapping is alive, the two mappings will alias
        /// each other, which is undefined behavior.
        ///
        /// Callers must therefore ensure no conflicting mapping of the same
        /// file can be created for as long as this mapping (and any borrowed
        /// slices it has yielded) lives.
        ///
        /// # Notes
        /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
        unsafe fn unlock(&mut self) -> Result<()>;

        /// Read bytes to the dst buf from the offset, returns how many bytes read.
        fn read(&self, dst: &mut [u8], offset: usize) -> usize {
          let buf = self.as_slice();

          if buf.len() < offset {
            0
          } else {
            let remaining = buf.len() - offset;
            let dst_len = dst.len();
            if remaining > dst_len {
              dst.copy_from_slice(&buf[offset..offset + dst_len]);
              dst_len
            } else {
              dst[..remaining].copy_from_slice(&buf[offset..offset + remaining]);
              remaining
            }
          }
        }

        /// Read the exact number of bytes required to fill buf.
        fn read_exact(&self, dst: &mut [u8], offset: usize) -> Result<()> {
          let buf = self.as_slice();
          let remaining = buf.len().checked_sub(offset);
          match remaining {
            None => Err(Error::from(ErrorKind::UnexpectedEof)),
            Some(remaining) => {
              let dst_len = dst.len();
              if remaining < dst_len {
                Err(Error::from(ErrorKind::UnexpectedEof))
              } else {
                dst.copy_from_slice(&buf[offset..offset + dst_len]);
                Ok(())
              }
            }
          }
        }

        /// Read a signed 8 bit integer from offset.
        fn read_i8(&self, offset: usize) -> Result<i8> {
          let buf = self.as_slice();

          let remaining = buf.len().checked_sub(offset);
          match remaining {
            None => Err(Error::from(ErrorKind::UnexpectedEof)),
            Some(remaining) => {
              if remaining < 1 {
                Err(Error::from(ErrorKind::UnexpectedEof))
              } else {
                Ok(buf[offset] as i8)
              }
            }
          }
        }

        /// Read a signed 16 bit integer from offset in big-endian byte order.
        fn read_i16(&self, offset: usize) -> Result<i16> {
          read_impl!(self, offset, i16::from_be_bytes)
        }

        /// Read a signed 16 bit integer from offset in little-endian byte order.
        fn read_i16_le(&self, offset: usize) -> Result<i16> {
          read_impl!(self, offset, i16::from_le_bytes)
        }

        /// Read a signed integer from offset in big-endian byte order.
        fn read_isize(&self, offset: usize) -> Result<isize> {
          read_impl!(self, offset, isize::from_be_bytes)
        }

        /// Read a signed integer from offset in little-endian byte order.
        fn read_isize_le(&self, offset: usize) -> Result<isize> {
          read_impl!(self, offset, isize::from_le_bytes)
        }

        /// Read a signed 32 bit integer from offset in big-endian byte order.
        fn read_i32(&self, offset: usize) -> Result<i32> {
          read_impl!(self, offset, i32::from_be_bytes)
        }

        /// Read a signed 32 bit integer from offset in little-endian byte order.
        fn read_i32_le(&self, offset: usize) -> Result<i32> {
          read_impl!(self, offset, i32::from_le_bytes)
        }

        /// Read a signed 64 bit integer from offset in big-endian byte order.
        fn read_i64(&self, offset: usize) -> Result<i64> {
          read_impl!(self, offset, i64::from_be_bytes)
        }

        /// Read a signed 64 bit integer from offset in little-endian byte order.
        fn read_i64_le(&self, offset: usize) -> Result<i64> {
          read_impl!(self, offset, i64::from_le_bytes)
        }

        /// Read a signed 128 bit integer from offset in big-endian byte order.
        fn read_i128(&self, offset: usize) -> Result<i128> {
          read_impl!(self, offset, i128::from_be_bytes)
        }

        /// Read a signed 128 bit integer from offset in little-endian byte order.
        fn read_i128_le(&self, offset: usize) -> Result<i128> {
          read_impl!(self, offset, i128::from_le_bytes)
        }

        /// Read an unsigned 8 bit integer from offset.
        fn read_u8(&self, offset: usize) -> Result<u8> {
          let buf = self.as_slice();

          let remaining = buf.len().checked_sub(offset);
          match remaining {
            None => Err(Error::from(ErrorKind::UnexpectedEof)),
            Some(remaining) => {
              if remaining < 1 {
                Err(Error::from(ErrorKind::UnexpectedEof))
              } else {
                Ok(buf[offset])
              }
            }
          }
        }

        /// Read an unsigned 16 bit integer from offset in big-endian.
        fn read_u16(&self, offset: usize) -> Result<u16> {
          read_impl!(self, offset, u16::from_be_bytes)
        }

        /// Read an unsigned 16 bit integer from offset in little-endian.
        fn read_u16_le(&self, offset: usize) -> Result<u16> {
          read_impl!(self, offset, u16::from_le_bytes)
        }

        /// Read an unsigned integer from offset in big-endian byte order.
        fn read_usize(&self, offset: usize) -> Result<usize> {
          read_impl!(self, offset, usize::from_be_bytes)
        }

        /// Read an unsigned integer from offset in little-endian byte order.
        fn read_usize_le(&self, offset: usize) -> Result<usize> {
          read_impl!(self, offset, usize::from_le_bytes)
        }

        /// Read an unsigned 32 bit integer from offset in big-endian.
        fn read_u32(&self, offset: usize) -> Result<u32> {
          read_impl!(self, offset, u32::from_be_bytes)
        }

        /// Read an unsigned 32 bit integer from offset in little-endian.
        fn read_u32_le(&self, offset: usize) -> Result<u32> {
          read_impl!(self, offset, u32::from_le_bytes)
        }

        /// Read an unsigned 64 bit integer from offset in big-endian.
        fn read_u64(&self, offset: usize) -> Result<u64> {
          read_impl!(self, offset, u64::from_be_bytes)
        }

        /// Read an unsigned 64 bit integer from offset in little-endian.
        fn read_u64_le(&self, offset: usize) -> Result<u64> {
          read_impl!(self, offset, u64::from_le_bytes)
        }

        /// Read an unsigned 128 bit integer from offset in big-endian.
        fn read_u128(&self, offset: usize) -> Result<u128> {
          read_impl!(self, offset, u128::from_be_bytes)
        }

        /// Read an unsigned 128 bit integer from offset in little-endian.
        fn read_u128_le(&self, offset: usize) -> Result<u128> {
          read_impl!(self, offset, u128::from_le_bytes)
        }

        /// Read an IEEE754 single-precision (4 bytes) floating point number from
        /// offset in big-endian byte order.
        fn read_f32(&self, offset: usize) -> Result<f32> {
          read_impl!(self, offset, f32::from_be_bytes)
        }

        /// Read an IEEE754 single-precision (4 bytes) floating point number from
        /// offset in little-endian byte order.
        fn read_f32_le(&self, offset: usize) -> Result<f32> {
          read_impl!(self, offset, f32::from_le_bytes)
        }

        /// Read an IEEE754 single-precision (8 bytes) floating point number from
        /// offset in big-endian byte order.
        fn read_f64(&self, offset: usize) -> Result<f64> {
          read_impl!(self, offset, f64::from_be_bytes)
        }

        /// Read an IEEE754 single-precision (8 bytes) floating point number from
        /// offset in little-endian byte order.
        fn read_f64_le(&self, offset: usize) -> Result<f64> {
          read_impl!(self, offset, f64::from_le_bytes)
        }
      }
    };
  }

  macro_rules! declare_async_mmap_file_mut_ext {
    ($writer: ty) => {
      /// Utility methods to [`AsyncMmapFileMut`]
      ///
      /// [`AsyncMmapFileMut`]: structs.AsyncMmapFileMut.html

      #[enum_dispatch]
      pub trait AsyncMmapFileMutExt {
        /// Returns the mutable underlying slice of the mmap
        fn as_mut_slice(&mut self) -> &mut [u8];

        /// slice_mut returns mutable data starting from offset off of size sz.
        ///
        /// # Panics
        /// If there's not enough data, or if `offset + sz` overflows, this panics.
        fn slice_mut(&mut self, offset: usize, sz: usize) -> &mut [u8] {
          let end = offset
            .checked_add(sz)
            .expect("offset + sz overflows usize");
          &mut self.as_mut_slice()[offset..end]
        }

        /// Whether mmap is copy on write
        fn is_cow(&self) -> bool;

        /// bytes_mut returns mutable data starting from offset off of size sz.
        ///
        /// # Errors
        /// If there's not enough data, it would return
        /// `Err(Error::from(ErrorKind::UnexpectedEof))`.
        fn bytes_mut(&mut self, offset: usize, sz: usize) -> Result<&mut [u8]> {
          let buf = self.as_mut_slice();
          crate::mmap_file::checked_range(offset, sz, buf.len())
            .map(|range| &mut buf[range])
        }

        /// Fill 0 to the specific range
        fn zero_range(&mut self, start: usize, end: usize) {
          let buf = self.as_mut_slice();
          let end = end.min(buf.len());
          buf[start..end].fill(0);
        }

        /// Flushes outstanding memory map modifications to disk (if the inner is a real file).
        ///
        /// When this method returns with a non-error result,
        /// all outstanding changes to a file-backed memory map are guaranteed to be durably stored.
        /// The file’s metadata (including last modification timestamp) may not be updated.
        fn flush(&self) -> Result<()>;

        /// Asynchronously flushes outstanding memory map modifications to disk(if the inner is a real file).
        ///
        /// This method initiates flushing modified pages to durable storage,
        /// but it will not wait for the operation to complete before returning.
        /// The file’s metadata (including last modification timestamp) may not be updated.
        fn flush_async(&self) -> Result<()>;

        /// Flushes outstanding memory map modifications in the range to disk(if the inner is a real file).
        ///
        /// The offset and length must be in the bounds of the memory map.
        ///
        /// When this method returns with a non-error result,
        /// all outstanding changes to a file-backed memory
        /// in the range are guaranteed to be durable stored.
        /// The file’s metadata (including last modification timestamp) may not be updated.
        /// It is not guaranteed the only the changes in the specified range are flushed;
        /// other outstanding changes to the memory map may be flushed as well.
        fn flush_range(&self, offset: usize, len: usize) -> Result<()>;

        /// Asynchronously flushes outstanding memory map modifications in the range to disk(if the inner is a real file).
        ///
        /// The offset and length must be in the bounds of the memory map.
        ///
        /// This method initiates flushing modified pages to durable storage,
        /// but it will not wait for the operation to complete before returning.
        /// The file’s metadata (including last modification timestamp) may not be updated.
        /// It is not guaranteed that the only changes flushed are those in the specified range;
        /// other outstanding changes to the memory map may be flushed as well.
        fn flush_async_range(&self, offset: usize, len: usize) -> Result<()>;

        /// Truncates the file to the `max_size`, which will lead to
        /// do re-mmap and sync_dir if the inner is a real file.
        fn truncate(&mut self, max_sz: u64) -> impl core::future::Future<Output = Result<()>> + Send;

        /// Remove the underlying file
        fn drop_remove(self) -> impl core::future::Future<Output = Result<()>> + Send;

        /// Close and truncate the underlying file
        fn close_with_truncate(self, max_sz: i64) -> impl core::future::Future<Output = Result<()>> + Send;

        /// Returns a [`AsyncMmapFileWriter`] base on the given `offset`, which helps read or write data from mmap like a normal File.
        ///
        /// # Notes
        /// If you use a writer to write data to mmap, there is no guarantee all
        /// data will be durably stored. So you need to call [`flush`]/[`flush_range`]/[`flush_async`]/[`flush_async_range`] in [`AsyncMmapFileMutExt`]
        /// to guarantee all data will be durably stored.
        ///
        /// # Errors
        /// If there's not enough data, it would return
        ///  `Err(Error::from(ErrorKind::UnexpectedEof))`.
        ///
        /// [`flush`]: traits.MmapFileMutExt.html#methods.flush
        /// [`flush_range`]: traits.MmapFileMutExt.html#methods.flush_range
        /// [`flush_async`]: traits.MmapFileMutExt.html#methods.flush_async
        /// [`flush_async_range`]: traits.MmapFileMutExt.html#methods.flush_async_range
        /// [`MmapFileWriter`]: structs.MmapFileWriter.html
        fn writer(&mut self, offset: usize) -> Result<$writer> {
          let buf = self.as_mut_slice();
          let buf_len = buf.len();
          if buf_len < offset {
            Err(Error::from(ErrorKind::UnexpectedEof))
          } else {
            Ok(<$writer>::new(Cursor::new(&mut buf[offset..]), offset, buf_len - offset))
          }
        }

        /// Returns a [`AsyncMmapFileWriter`] base on the given `offset` and `len`, which helps read or write data from mmap like a normal File.
        ///
        /// # Notes
        /// If you use a writer to write data to mmap, there is no guarantee all
        /// data will be durably stored. So you need to call [`flush`]/[`flush_range`]/[`flush_async`]/[`flush_async_range`] in [`MmapFileMutExt`]
        /// to guarantee all data will be durably stored.
        ///
        /// # Errors
        /// If there's not enough data, it would return
        ///  `Err(Error::from(ErrorKind::UnexpectedEof))`.
        ///
        /// [`flush`]: traits.AsyncMmapFileMutExt.html#methods.flush
        /// [`flush_range`]: traits.AsyncMmapFileMutExt.html#methods.flush_range
        /// [`flush_async`]: traits.AsyncMmapFileMutExt.html#methods.flush_async
        /// [`flush_async_range`]: traits.AsyncMmapFileMutExt.html#methods.flush_async_range
        /// [`AsyncMmapFileWriter`]: structs.AsyncMmapFileWriter.html
        fn range_writer(&mut self, offset: usize, len: usize) -> Result<$writer> {
          let buf = self.as_mut_slice();
          let range = crate::mmap_file::checked_range(offset, len, buf.len())?;
          Ok(<$writer>::new(Cursor::new(&mut buf[range]), offset, len))
        }

        /// Write bytes to the mmap from the offset.
        fn write(&mut self, src: &[u8], offset: usize) -> usize {
          let buf = self.as_mut_slice();
          if buf.len() <= offset {
            0
          } else {
            let remaining = buf.len() - offset;
            let src_len = src.len();
            if remaining > src_len {
              buf[offset..offset + src_len].copy_from_slice(src);
              src_len
            } else {
              buf[offset..offset + remaining].copy_from_slice(&src[..remaining]);
              remaining
            }
          }
        }

        /// Write the all of bytes in `src` to the mmap from the offset.
        fn write_all(&mut self, src: &[u8], offset: usize) -> Result<()> {
          let buf = self.as_mut_slice();
          let remaining = buf.len().checked_sub(offset);
          match remaining {
            None => Err(Error::from(ErrorKind::UnexpectedEof)),
            Some(remaining) => {
              let src_len = src.len();
              if remaining < src_len {
                Err(Error::from(ErrorKind::UnexpectedEof))
              } else {
                buf[offset..offset + src_len].copy_from_slice(src);
                Ok(())
              }
            }
          }
        }

        /// Writes a signed 8 bit integer to mmap from the offset.
        fn write_i8(&mut self, val: i8, offset: usize) -> Result<()> {
          self.write_all(&[val as u8], offset)
        }

        /// Writes a signed 16 bit integer to mmap from the offset in the big-endian byte order.
        fn write_i16(&mut self, val: i16, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes a signed 16 bit integer to mmap from the offset in the little-endian byte order.
        fn write_i16_le(&mut self, val: i16, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes a signed integer to mmap from the offset in the big-endian byte order.
        fn write_isize(&mut self, val: isize, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes a signed integer to mmap from the offset in the little-endian byte order.
        fn write_isize_le(&mut self, val: isize, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes a signed 32 bit integer to mmap from the offset in the big-endian byte order.
        fn write_i32(&mut self, val: i32, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes a signed 32 bit integer to mmap from the offset in the little-endian byte order.
        fn write_i32_le(&mut self, val: i32, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes a signed 64 bit integer to mmap from the offset in the big-endian byte order.
        fn write_i64(&mut self, val: i64, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes a signed 64 bit integer to mmap from the offset in the little-endian byte order.
        fn write_i64_le(&mut self, val: i64, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes a signed 128 bit integer to mmap from the offset in the big-endian byte order.
        fn write_i128(&mut self, val: i128, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes a signed 128 bit integer to mmap from the offset in the little-endian byte order.
        fn write_i128_le(&mut self, val: i128, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an unsigned 8 bit integer to mmap from the offset.
        fn write_u8(&mut self, val: u8, offset: usize) -> Result<()> {
          self.write_all(&[val], offset)
        }

        /// Writes an unsigned 16 bit integer to mmap from the offset in the big-endian byte order.
        fn write_u16(&mut self, val: u16, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an unsigned 16 bit integer to mmap from the offset in the little-endian byte order.
        fn write_u16_le(&mut self, val: u16, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an unsigned integer to mmap from the offset in the big-endian byte order.
        fn write_usize(&mut self, val: usize, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an unsigned integer to mmap from the offset in the little-endian byte order.
        fn write_usize_le(&mut self, val: usize, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an unsigned 32 bit integer to mmap from the offset in the big-endian byte order.
        fn write_u32(&mut self, val: u32, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an unsigned 32 bit integer to mmap from the offset in the little-endian byte order.
        fn write_u32_le(&mut self, val: u32, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an unsigned 64 bit integer to mmap from the offset in the big-endian byte order.
        fn write_u64(&mut self, val: u64, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an unsigned 64 bit integer to mmap from the offset in the little-endian byte order.
        fn write_u64_le(&mut self, val: u64, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an unsigned 128 bit integer to mmap from the offset in the big-endian byte order.
        fn write_u128(&mut self, val: u128, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an unsigned 128 bit integer to mmap from the offset in the little-endian byte order.
        fn write_u128_le(&mut self, val: u128, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an IEEE754 single-precision (4 bytes) floating point number to mmap from the offset in big-endian byte order.
        fn write_f32(&mut self, val: f32, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an IEEE754 single-precision (4 bytes) floating point number to mmap from the offset in little-endian byte order.
        fn write_f32_le(&mut self, val: f32, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an IEEE754 single-precision (8 bytes) floating point number to mmap from the offset in big-endian byte order.
        fn write_f64(&mut self, val: f64, offset: usize) -> Result<()> {
          self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an IEEE754 single-precision (8 bytes) floating point number to mmap from the offset in little-endian byte order.
        fn write_f64_le(&mut self, val: f64, offset: usize) -> Result<()> {
          self.write_all(&val.to_le_bytes(), offset)
        }
      }
    };
  }

  macro_rules! declare_and_impl_inners {
    () => {
      enum AsyncMmapFileInner {
        Empty(AsyncEmptyMmapFile),
        Memory(AsyncMemoryMmapFile),
        Disk(AsyncDiskMmapFile)
      }

      impl From<AsyncEmptyMmapFile> for AsyncMmapFileInner {
        fn from(v: AsyncEmptyMmapFile) -> AsyncMmapFileInner {
          AsyncMmapFileInner::Empty(v)
        }
      }
      impl From<AsyncMemoryMmapFile> for AsyncMmapFileInner {
        fn from(v: AsyncMemoryMmapFile) -> AsyncMmapFileInner {
          AsyncMmapFileInner::Memory(v)
        }
      }
      impl From<AsyncDiskMmapFile> for AsyncMmapFileInner {
        fn from(v: AsyncDiskMmapFile) -> AsyncMmapFileInner {
          AsyncMmapFileInner::Disk(v)
        }
      }


      impl AsyncMmapFileExt for AsyncMmapFileInner {
        #[inline]
        fn len(&self) -> usize {
          match self {
            AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::len(inner),
            AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::len(inner),
            AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::len(inner),
          }
        }

        #[inline]
        fn as_slice(&self) -> &[u8] {
          match self {
            AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::as_slice(inner),
            AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::as_slice(inner),
            AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::as_slice(inner),
          }
        }

        #[inline]
        fn path(&self) -> &Path {
          match self {
            AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::path(inner),
            AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::path(inner),
            AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::path(inner),
          }
        }

        #[inline]
        fn is_exec(&self) -> bool {
          match self {
            AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::is_exec(inner),
            AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::is_exec(inner),
            AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::is_exec(inner),
          }
        }

        #[inline]
        async fn metadata(&self) -> Result<MetaData> {
          match self {
            AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::metadata(inner).await,
            AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::metadata(inner).await,
            AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::metadata(inner).await,
          }
        }

        #[inline]
        fn lock(&mut self) -> Result<()> {
          match self {
            AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::lock(inner),
            AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::lock(inner),
            AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::lock(inner),
          }
        }

        #[inline]
        unsafe fn lock_shared(&mut self) -> Result<()> {
          match self {
            AsyncMmapFileInner::Empty(inner) => unsafe { AsyncMmapFileExt::lock_shared(inner) },
            AsyncMmapFileInner::Memory(inner) => unsafe { AsyncMmapFileExt::lock_shared(inner) },
            AsyncMmapFileInner::Disk(inner) => unsafe { AsyncMmapFileExt::lock_shared(inner) },
          }
        }

        #[inline]
        fn try_lock(&mut self) -> Result<()> {
          match self {
            AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::try_lock(inner),
            AsyncMmapFileInner::Memory(inner) => {
              AsyncMmapFileExt::try_lock(inner)
            }
            AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::try_lock(inner),
          }
        }

        #[inline]
        unsafe fn try_lock_shared(&mut self) -> Result<()> {
          match self {
            AsyncMmapFileInner::Empty(inner) => unsafe { AsyncMmapFileExt::try_lock_shared(inner) },
            AsyncMmapFileInner::Memory(inner) => unsafe { AsyncMmapFileExt::try_lock_shared(inner) },
            AsyncMmapFileInner::Disk(inner) => unsafe { AsyncMmapFileExt::try_lock_shared(inner) },
          }
        }

        #[inline]
        unsafe fn unlock(&mut self) -> Result<()> {
          match self {
            AsyncMmapFileInner::Empty(inner) => unsafe { AsyncMmapFileExt::unlock(inner) },
            AsyncMmapFileInner::Memory(inner) => unsafe { AsyncMmapFileExt::unlock(inner) },
            AsyncMmapFileInner::Disk(inner) => unsafe { AsyncMmapFileExt::unlock(inner) },
          }
        }
      }

      enum AsyncMmapFileMutInner {
        Empty(AsyncEmptyMmapFile),
        Memory(AsyncMemoryMmapFileMut),
        Disk(AsyncDiskMmapFileMut)
      }

      impl From<AsyncEmptyMmapFile> for AsyncMmapFileMutInner {
        fn from(v: AsyncEmptyMmapFile) -> AsyncMmapFileMutInner {
          AsyncMmapFileMutInner::Empty(v)
        }
      }
      impl From<AsyncMemoryMmapFileMut> for AsyncMmapFileMutInner {
        fn from(v: AsyncMemoryMmapFileMut) -> AsyncMmapFileMutInner {
          AsyncMmapFileMutInner::Memory(v)
        }
      }
      impl From<AsyncDiskMmapFileMut> for AsyncMmapFileMutInner {
        fn from(v: AsyncDiskMmapFileMut) -> AsyncMmapFileMutInner {
          AsyncMmapFileMutInner::Disk(v)
        }
      }


      impl AsyncMmapFileExt for AsyncMmapFileMutInner {
        #[inline]
        fn len(&self) -> usize {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::len(inner),
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::len(inner),
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::len(inner),
          }
        }

        #[inline]
        fn as_slice(&self) -> &[u8] {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::as_slice(inner),
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::as_slice(inner),
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::as_slice(inner),
          }
        }

        #[inline]
        fn path(&self) -> &Path {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::path(inner),
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::path(inner),
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::path(inner),
          }
        }

        #[inline]
        fn is_exec(&self) -> bool {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::is_exec(inner),
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::is_exec(inner),
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::is_exec(inner),
          }
        }

        #[inline]
        async fn metadata(&self) -> Result<MetaData> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::metadata(inner).await,
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::metadata(inner).await,
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::metadata(inner).await,
          }
        }

        #[inline]
        fn lock(&mut self) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::lock(inner),
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::lock(inner),
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::lock(inner),
          }
        }

        #[inline]
        unsafe fn lock_shared(&mut self) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => unsafe { AsyncMmapFileExt::lock_shared(inner) },
            AsyncMmapFileMutInner::Memory(inner) => unsafe { AsyncMmapFileExt::lock_shared(inner) },
            AsyncMmapFileMutInner::Disk(inner) => unsafe { AsyncMmapFileExt::lock_shared(inner) },
          }
        }

        #[inline]
        fn try_lock(&mut self) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::try_lock(inner),
            AsyncMmapFileMutInner::Memory(inner) => {
              AsyncMmapFileExt::try_lock(inner)
            }
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::try_lock(inner),
          }
        }

        #[inline]
        unsafe fn try_lock_shared(&mut self) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => unsafe { AsyncMmapFileExt::try_lock_shared(inner) },
            AsyncMmapFileMutInner::Memory(inner) => unsafe { AsyncMmapFileExt::try_lock_shared(inner) },
            AsyncMmapFileMutInner::Disk(inner) => unsafe { AsyncMmapFileExt::try_lock_shared(inner) },
          }
        }

        #[inline]
        unsafe fn unlock(&mut self) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => unsafe { AsyncMmapFileExt::unlock(inner) },
            AsyncMmapFileMutInner::Memory(inner) => unsafe { AsyncMmapFileExt::unlock(inner) },
            AsyncMmapFileMutInner::Disk(inner) => unsafe { AsyncMmapFileExt::unlock(inner) },
          }
        }
      }


      impl AsyncMmapFileMutExt for AsyncMmapFileMutInner {
        #[inline]
        fn as_mut_slice(&mut self) -> &mut [u8] {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::as_mut_slice(inner),
            AsyncMmapFileMutInner::Memory(inner) => {
              AsyncMmapFileMutExt::as_mut_slice(inner)
            }
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::as_mut_slice(inner),
          }
        }

        #[inline]
        fn is_cow(&self) -> bool {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::is_cow(inner),
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::is_cow(inner),
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::is_cow(inner),
          }
        }

        #[inline]
        fn flush(&self) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::flush(inner),
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::flush(inner),
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::flush(inner),
          }
        }

        #[inline]
        fn flush_async(&self) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::flush_async(inner),
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::flush_async(inner),
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::flush_async(inner),
          }
        }

        #[inline]
        fn flush_range(&self, offset: usize, len: usize) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::flush_range(
              inner,
              offset,
              len,
            ),
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::flush_range(
              inner,
              offset,
              len,
            ),
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::flush_range(
              inner,
              offset,
              len,
            ),
          }
        }

        #[inline]
        fn flush_async_range(&self, offset: usize, len: usize) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::flush_async_range(
              inner,
              offset,
              len,
            ),
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::flush_async_range(
              inner,
              offset,
              len,
            ),
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::flush_async_range(
              inner,
              offset,
              len,
            ),
          }
        }

        async fn truncate(&mut self, max_sz: u64) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => {
              AsyncMmapFileMutExt::truncate(inner, max_sz).await
            }
            AsyncMmapFileMutInner::Memory(inner) => {
              AsyncMmapFileMutExt::truncate(inner, max_sz).await
            }
            AsyncMmapFileMutInner::Disk(inner) => {
              AsyncMmapFileMutExt::truncate(inner, max_sz).await
            }
          }
        }

        async fn drop_remove(self) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::drop_remove(inner).await,
            AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::drop_remove(inner).await,
            AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::drop_remove(inner).await,
          }
        }

        async fn close_with_truncate(self, max_sz: i64) -> Result<()> {
          match self {
            AsyncMmapFileMutInner::Empty(inner) => {
              AsyncMmapFileMutExt::close_with_truncate(inner, max_sz).await
            }
            AsyncMmapFileMutInner::Memory(inner) => {
              AsyncMmapFileMutExt::close_with_truncate(inner, max_sz).await
            }
            AsyncMmapFileMutInner::Disk(inner) => {
              AsyncMmapFileMutExt::close_with_truncate(inner, max_sz).await
            }
          }
        }
      }

    };
  }


  macro_rules! declare_and_impl_async_mmap_file {
    ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal) => {
      /// A read-only memory map file.
      ///
      /// There is 3 status of this struct:
      /// - __Disk__: mmap to a real file
      /// - __Memory__: use [`Bytes`] to mock a mmap, which is useful for test and in-memory storage engine
      /// - __Empty__: a state represents null mmap, which is helpful for drop, close the `AsyncMmapFile`. This state cannot be constructed directly.
      ///
      /// [`Bytes`]: https://docs.rs/bytes/1.1.0/bytes/struct.Bytes.html
      #[repr(transparent)]
      pub struct AsyncMmapFile {
        inner: AsyncMmapFileInner
      }

      impl_from!(AsyncMmapFile, AsyncMmapFileInner, [AsyncEmptyMmapFile, AsyncMemoryMmapFile, AsyncDiskMmapFile]);

      impl_async_mmap_file_ext!(AsyncMmapFile);

      impl AsyncMmapFile {
        /// Open a readable memory map backed by a file
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFile, AsyncMmapFileExt};")]
        #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
        #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_open_test.txt\").unwrap());")]
        /// # file.truncate(12).await.unwrap();
        /// # file.write_all("some data...".as_bytes(), 0).unwrap();
        /// # file.flush().unwrap();
        /// # drop(file);
        /// // mmap the file
        #[doc = concat!("let mut file = AsyncMmapFile::open(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
        /// let mut buf = vec![0; "some data...".len()];
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        /// # })
        #[doc = "```"]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFile::open(path).await?))
        }

        /// Open a readable memory map backed by a file with [`Options`]
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFile, AsyncMmapFileExt};")]
        #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
        #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_open_with_options_test.txt\").unwrap());")]
        /// # file.truncate(23).await.unwrap();
        /// # file.write_all("sanity text".as_bytes(), 0).unwrap();
        /// # file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
        /// # file.flush().unwrap();
        /// # drop(file);
        ///
        /// // mmap the file
        /// let opts = AsyncOptions::new()
        ///     // mmap content after the sanity text
        ///     .offset("sanity text".as_bytes().len() as u64);
        /// // mmap the file
        #[doc = concat!("let mut file = AsyncMmapFile::open_with_options(\"", $filename_prefix, "_open_with_options_test.txt\", opts).await.unwrap();")]
        /// let mut buf = vec![0; "some data...".len()];
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFile::open_with_options(path, opts).await?))
        }

        /// Open a readable and executable memory map backed by a file
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFile, AsyncMmapFileExt};")]
        #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_exec_test.txt\").await.unwrap();")]
        #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_open_exec_test.txt\").unwrap());")]
        /// # file.truncate(12).await.unwrap();
        /// # file.write_all("some data...".as_bytes(), 0).unwrap();
        /// # file.flush().unwrap();
        /// # drop(file);
        /// // mmap the file
        #[doc = concat!("let mut file = AsyncMmapFile::open_exec(\"", $filename_prefix, "_open_exec_test.txt\").await.unwrap();")]
        /// let mut buf = vec![0; "some data...".len()];
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        /// # })
        #[doc = "```"]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_exec<P: AsRef<Path>>(path: P) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFile::open_exec(path).await?))
        }

        /// Open a readable and executable memory map backed by a file with [`Options`].
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFile, AsyncMmapFileExt};")]
        #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_exec_with_options_test.txt\").await.unwrap();")]
        #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_open_exec_with_options_test.txt\").unwrap());")]
        /// # file.truncate(23).await.unwrap();
        /// # file.write_all("sanity text".as_bytes(), 0).unwrap();
        /// # file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
        /// # file.flush().unwrap();
        /// # drop(file);
        ///
        /// // mmap the file
        /// let opts = AsyncOptions::new()
        ///     // mmap content after the sanity text
        ///     .offset("sanity text".as_bytes().len() as u64);
        /// // mmap the file
        #[doc = concat!("let mut file = AsyncMmapFile::open_exec_with_options(\"", $filename_prefix, "_open_exec_with_options_test.txt\", opts).await.unwrap();")]
        /// let mut buf = vec![0; "some data...".len()];
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_exec_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFile::open_exec_with_options(path, opts).await?))
        }
      }

      impl_constructor_for_memory_mmap_file!(AsyncMemoryMmapFile, AsyncMmapFile, "AsyncMmapFile", $path_str);
    };
  }

  macro_rules! delcare_and_impl_async_mmap_file_mut {
    ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal) => {
      /// Initial-call durable unlink: see sync sibling
      /// `initial_remove_durably`. Identity-check first, then unlink. The
      /// first unlink runs after the wrapper has dropped its handle, so a
      /// concurrent rename + recreate could already have happened — the
      /// pre-unlink identity check refuses to delete a different file at
      /// the same path.
      /// `inode_pin` mirrors the sync sibling: a `std::fs::File`
      /// duplicated from the original async file descriptor (POSIX) so
      /// the inode it refers to cannot be recycled while the identity
      /// probe + unlink run. Windows callers pass `None`.
      ///
      /// The entire blocking sequence (open parent, fstatat, unlinkat,
      /// fsync) runs through the runtime's blocking-task pool
      /// (`tokio::task::spawn_blocking` / `smol::unblock`) so a slow
      /// filesystem can't stall the async executor.
      async fn initial_remove_durably_async(
        path: &::std::path::Path,
        identity: crate::utils::FileIdentity,
        #[cfg(unix)] inode_pin: std::fs::File,
      ) -> ::std::result::Result<
        (),
        (crate::mmap_file::PendingDelete, Error),
      > {
        // pin is required (not Option) on POSIX. Callers must dup
        // before calling and hard-fail on dup failure. The pin is
        // moved into `PendingDelete::NeedsUnlink` on retryable
        // failures so retries inherit it; otherwise dropped when the
        // closure exits.
        let path = path.to_path_buf();
        run_blocking_io(move || -> ::std::result::Result<(), (crate::mmap_file::PendingDelete, Error)> {
          #[cfg(unix)]
          let inode_pin = inode_pin;
          let parent_handle = match crate::utils::open_parent_for_sync(&path) {
            Ok(h) => h,
            Err(e) => {
              return Err((
                crate::mmap_file::PendingDelete::NeedsUnlink {
                  path: path.clone(),
                  identity,
                  #[cfg(unix)]
                  pin: inode_pin,
                },
                e,
              ));
            }
          };
          match crate::utils::identity_at_or_path(&parent_handle, &path) {
            Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => {
              // See sync `initial_remove_durably` for the rationale.
              // NotFound never proves crash-durable deletion
              // (rename-then-unlink-elsewhere produces nlink==0 too,
              // but the entry was removed from a parent we don't
              // fsync). Always NeedsUnlink.
              return Err((
                crate::mmap_file::PendingDelete::NeedsUnlink {
                  path: path.clone(),
                  identity,
                  #[cfg(unix)]
                  pin: inode_pin,
                },
                e,
              ));
            }
            Err(e) => {
              return Err((
                crate::mmap_file::PendingDelete::NeedsUnlink {
                  path: path.clone(),
                  identity,
                  #[cfg(unix)]
                  pin: inode_pin,
                },
                e,
              ));
            }
            Ok(probe_id) => {
              if !identity.is_known_equal(&probe_id) {
                let err = Error::other(format!(
                  "cannot unlink '{}': path no longer names the original file (path-reuse detected between handle drop and unlink)",
                  path.display(),
                ));
                return Err((
                  crate::mmap_file::PendingDelete::NeedsUnlink {
                    path: path.clone(),
                    identity,
                    #[cfg(unix)]
                    pin: inode_pin,
                  },
                  err,
                ));
              }
            }
          }
          match crate::utils::unlink_at_or_path(&parent_handle, &path, identity) {
            Ok(()) => match crate::utils::sync_parent_handle(&parent_handle) {
              Ok(()) => Ok(()),
              Err(e) => Err((
                crate::mmap_file::PendingDelete::NeedsParentSync {
                  path: path.clone(),
                  parent_handle,
                },
                e,
              )),
            },
            Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => {
              // NotFound from `unlinkat` after a matching probe could
              // still be a rename-then-unlink-elsewhere. We can't
              // claim our parent fsync makes anything crash-durable.
              // Stay NeedsUnlink.
              Err((
                crate::mmap_file::PendingDelete::NeedsUnlink {
                  path: path.clone(),
                  identity,
                  #[cfg(unix)]
                  pin: inode_pin,
                },
                e,
              ))
            }
            Err(e) => Err((
              crate::mmap_file::PendingDelete::NeedsUnlink {
                path: path.clone(),
                identity,
                #[cfg(unix)]
                pin: inode_pin,
              },
              e,
            )),
          }
        })
        .await
      }

      /// Retry a pending delete in a path-reuse-safe way.
      ///
      /// `NeedsParentSync` only fsyncs the parent — never re-calls
      /// `remove_file`. `NeedsUnlink` delegates to
      /// `initial_remove_durably_async` so the nlink classification
      /// (nlink==0 → NeedsParentSync, nlink>0 → NeedsUnlink) and
      /// NotFound handling apply uniformly to both initial and retry
      /// paths. Previously this branch open-coded a probe-then-unlink
      /// that converted every probe error into a path-reuse refusal,
      /// missing the "renamed away then unlinked at the new name"
      /// case (pin nlink==0 → should converge to NeedsParentSync).
      async fn retry_pending_delete_async(
        pending: crate::mmap_file::PendingDelete,
      ) -> ::std::result::Result<
        (),
        (crate::mmap_file::PendingDelete, Error),
      > {
        match pending {
          crate::mmap_file::PendingDelete::NeedsParentSync {
            path,
            parent_handle,
          } => {
            run_blocking_io(move || -> ::std::result::Result<(), (crate::mmap_file::PendingDelete, Error)> {
              match crate::utils::sync_parent_handle(&parent_handle) {
                Ok(()) => Ok(()),
                Err(e) => Err((
                  crate::mmap_file::PendingDelete::NeedsParentSync {
                    path,
                    parent_handle,
                  },
                  e,
                )),
              }
            })
            .await
          }
          crate::mmap_file::PendingDelete::NeedsUnlink {
            path,
            identity,
            #[cfg(unix)]
            pin,
          } => {
            initial_remove_durably_async(
              &path,
              identity,
              #[cfg(unix)]
              pin,
            )
            .await
          }
        }
      }

      /// A writable memory map file.
      ///
      /// There is 3 status of this struct:
      /// - __Disk__: mmap to a real file
      /// - __Memory__: use [`BytesMut`] to mock a mmap, which is useful for test and in-memory storage engine
      /// - __Empty__: a state represents null mmap, which is helpful for drop, remove, close the `AsyncMmapFileMut`. This state cannot be constructed directly.
      ///
      /// [`BytesMut`]: https://docs.rs/bytes/1.1.0/bytes/struct.BytesMut.html
      pub struct AsyncMmapFileMut {
        inner: AsyncMmapFileMutInner,
        remove_on_drop: bool,
        deleted: bool,
        /// User-requested deletion that failed and must be retried on
        /// `Drop`. See `PendingDelete` for the path-reuse-safety variant
        /// distinction.
        pending_drop_remove: Option<crate::mmap_file::PendingDelete>,
        /// Path retained so `Drop`'s opt-in `remove_on_drop` cleanup has a
        /// target after the inner mapping was already dropped — e.g.
        /// consuming `close_with_truncate(self)` failed mid-way and the
        /// inner is now `Empty`.
        pending_remove_path: Option<std::path::PathBuf>,
      }

      impl_from_mut!(AsyncMmapFileMut, AsyncMmapFileMutInner, [AsyncEmptyMmapFile, AsyncMemoryMmapFileMut, AsyncDiskMmapFileMut]);

      impl_async_mmap_file_ext!(AsyncMmapFileMut);

      impl_async_mmap_file_mut_ext!($filename_prefix, $doc_test_runtime, $path_str);

      impl AsyncMmapFileMut {
        /// Create a new file and mmap this file
        ///
        /// # Notes
        /// The new file is zero size, so, before write, you should truncate first.
        /// Or you can use [`create_with_options`] and set `max_size` field for [`AsyncOptions`] to enable directly write
        /// without truncating.
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_create_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_create_test.txt\").unwrap());")]
        /// file.truncate(12).await;
        /// file.write_all("some data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[`create_with_options`]: ", $path_str, "/struct.AsyncMmapFileMut.html#method.create_with_options")]
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFileMut::create(path).await?))
        }

        /// Create a new file and mmap this file with [`AsyncOptions`]
        ///
        /// # Example
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        /// let opts = AsyncOptions::new()
        ///     // truncate to 100
        ///     .max_size(100);
        #[doc = concat!("let mut file = AsyncMmapFileMut::create_with_options(\"", $filename_prefix, "_create_with_options_test.txt\", opts).await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_create_with_options_test.txt\").unwrap());")]
        /// file.write_all("some data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn create_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFileMut::create_with_options(path, opts).await?))
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
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_test.txt\").unwrap());")]
        /// # file.truncate(12).await.unwrap();
        /// # file.write_all("some data...".as_bytes(), 0).unwrap();
        /// # file.flush().unwrap();
        /// # drop(file);
        ///
        /// // mmap the file
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
        /// let mut buf = vec![0; "some data...".len()];
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        ///
        /// // modify the file data
        /// file.truncate("some modified data...".len() as u64).await.unwrap();
        /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        /// drop(file);
        ///
        /// // reopen to check content
        /// let mut buf = vec![0; "some modified data...".len()];
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
        /// # })
        #[doc = "```"]
        ///
        /// File does not exists
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        /// // mmap the file
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_test.txt\").unwrap());")]
        /// file.truncate(100).await.unwrap();
        /// file.write_all("some data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        ///
        /// let mut buf = vec![0; "some data...".len()];
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        ///
        /// // modify the file data
        /// file.truncate("some modified data...".len() as u64).await.unwrap();
        /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        /// drop(file);
        ///
        /// // reopen to check content
        /// let mut buf = vec![0; "some modified data...".len()];
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[`open_with_options`]: ", $path_str, "/struct.AsyncMmapFileMut.html#method.open_with_options")]
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFileMut::open(path).await?))
        }

        /// Open or Create(if not exists) a file and mmap this file with [`AsyncOptions`].
        ///
        /// # Examples
        ///
        /// File already exists
        ///
        /// ```ignore
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_with_options_test.txt\").unwrap());")]
        /// # file.truncate(23).await.unwrap();
        /// # file.write_all("sanity text".as_bytes(), 0).unwrap();
        /// # file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
        /// # file.flush().unwrap();
        /// # drop(file);
        ///
        /// // mmap the file
        /// let opts = AsyncOptions::new()
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
        #[doc = concat!("let mut file = AsyncMmapFileMut::open_with_options(\"", $filename_prefix, "_open_with_options_test.txt\", opts).await.unwrap();")]
        /// let mut buf = vec![0; "some data...".len()];
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        ///
        /// // modify the file data
        /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).await.unwrap();
        /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        /// drop(file);
        ///
        /// // reopen to check content
        /// let mut buf = vec![0; "some modified data...".len()];
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
        /// // skip the sanity text
        /// file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
        /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
        /// # })
        #[doc = "```"]
        ///
        /// File does not exists
        ///
        /// ```ignore
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        /// // mmap the file with options
        /// let opts = AsyncOptions::new()
        ///     // allow read
        ///     .read(true)
        ///     // allow write
        ///     .write(true)
        ///     // allow append
        ///     .append(true)
        ///     // truncate to 100
        ///     .max_size(100);
        ///
        #[doc = concat!("let mut file = AsyncMmapFileMut::open_with_options(\"", $filename_prefix, "_open_with_options_test.txt\", opts).await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_with_options_test.txt\").unwrap());")]
        /// file.write_all("some data...".as_bytes(), 0).unwrap();
        ///
        /// let mut buf = vec![0; "some data...".len()];
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        ///
        /// // modify the file data
        /// file.truncate("some modified data...".len() as u64).await.unwrap();
        /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        /// drop(file);
        ///
        /// // reopen to check content
        /// let mut buf = vec![0; "some modified data...".len()];
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFileMut::open_with_options(path, opts).await?))
        }

        /// Open an existing file and mmap this file
        ///
        /// # Examples
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        /// // create a temp file
        #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_existing_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_existing_test.txt\").unwrap());")]
        /// file.truncate(12).await.unwrap();
        /// file.write_all("some data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        /// drop(file);
        ///
        /// // mmap the file
        #[doc = concat!("let mut file = AsyncMmapFileMut::open_exist(\"", $filename_prefix, "_open_existing_test.txt\").await.unwrap();")]
        /// let mut buf = vec![0; "some data...".len()];
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        ///
        /// // modify the file data
        /// file.truncate("some modified data...".len() as u64).await.unwrap();
        /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        /// drop(file);
        ///
        ///
        /// // reopen to check content
        /// let mut buf = vec![0; "some modified data...".len()];
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_existing_test.txt\").await.unwrap();")]
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFileMut::open_exist(path).await?))
        }

        /// Open an existing file and mmap this file with [`AsyncOptions`]
        ///
        /// # Examples
        ///
        /// ```ignore
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        /// // create a temp file
        #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_existing_test_with_options.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_existing_test_with_options.txt\").unwrap());")]
        /// file.truncate(23).await.unwrap();
        /// file.write_all("sanity text".as_bytes(), 0).unwrap();
        /// file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
        /// file.flush().unwrap();
        /// drop(file);
        ///
        /// // mmap the file
        /// let opts = AsyncOptions::new()
        ///     // truncate to 100
        ///     .max_size(100)
        ///     // mmap content after the sanity text
        ///     .offset("sanity text".as_bytes().len() as u64);
        ///
        #[doc = concat!("let mut file = AsyncMmapFileMut::open_exist_with_options(\"", $filename_prefix, "_open_existing_test_with_options.txt\", opts).await.unwrap();")]
        ///
        /// let mut buf = vec![0; "some data...".len()];
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        ///
        /// // modify the file data
        /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).await.unwrap();
        /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        ///
        /// // reopen to check content, cow will not change the content.
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_existing_test_with_options.txt\").await.unwrap();")]
        /// let mut buf = vec![0; "some modified data...".len()];
        /// // skip the sanity text
        /// // file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
        /// // assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFileMut::open_exist_with_options(path, opts).await?))
        }

        /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file).
        /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        /// // create a temp file
        #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_cow_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_cow_test.txt\").unwrap());")]
        /// file.truncate(12).await.unwrap();
        /// file.write_all("some data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        /// drop(file);
        ///
        /// // mmap the file
        #[doc = concat!("let mut file = AsyncMmapFileMut::open_cow(\"", $filename_prefix, "_open_cow_test.txt\").await.unwrap();")]
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
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_cow_test.txt\").await.unwrap();")]
        /// let mut buf = vec![0; "some data...".len()];
        /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFileMut::open_cow(path).await?))
        }

        /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file) with [`AsyncOptions`].
        /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
        /// use std::io::SeekFrom;
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        /// // create a temp file
        #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_cow_with_options_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_cow_with_options_test.txt\").unwrap());")]
        /// file.truncate(23).await.unwrap();
        /// file.write_all("sanity text".as_bytes(), 0).unwrap();
        /// file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
        /// file.flush().unwrap();
        /// drop(file);
        ///
        /// // mmap the file
        /// let opts = AsyncOptions::new()
        ///     // mmap content after the sanity text
        ///     .offset("sanity text".as_bytes().len() as u64);
        ///
        #[doc = concat!("let mut file = AsyncMmapFileMut::open_cow_with_options(\"", $filename_prefix, "_open_cow_with_options_test.txt\", opts).await.unwrap();")]
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
        #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_cow_with_options_test.txt\").await.unwrap();")]
        /// let mut buf = vec![0; "some data...".len()];
        /// // skip the sanity text
        /// file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
        /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
        #[doc = ""]
        #[doc = "# Safety"]
        #[doc = ""]
        #[doc = "See the [crate-level safety section](crate) for the full contract."]
        pub async unsafe fn open_cow_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
          Ok(Self::from(AsyncDiskMmapFileMut::open_cow_with_options(path, opts).await?))
        }

        /// Make the mmap file read-only.
        ///
        /// # Notes
        /// If `remove_on_drop` is set to `true`, then the underlying file will not be removed on drop if this function is invoked. [Read more]
        ///
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
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_freeze_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_freeze_test.txt\").unwrap());")]
        /// file.truncate(12).await;
        /// file.write_all("some data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        /// // freeze
        /// file.freeze().unwrap();
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[Read more]: ", $path_str, "/struct.AsyncMmapFileMut.html#methods.set_remove_on_drop")]
        ///
        #[inline]
        pub fn freeze(mut self) -> Result<AsyncMmapFile> {
          let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
          let inner = mem::replace(&mut self.inner, empty);
          let path = inner.path_buf();
          match inner {
            AsyncMmapFileMutInner::Empty(empty) => Ok(AsyncMmapFile::from(empty)), // unreachable, keep this for good measure
            AsyncMmapFileMutInner::Memory(memory) => Ok(AsyncMmapFile::from(memory.freeze())),
            AsyncMmapFileMutInner::Disk(disk) => match disk.freeze() {
              Ok(frozen) => Ok(AsyncMmapFile::from(frozen)),
              Err(e) => {
                if !path.as_os_str().is_empty() {
                  self.pending_remove_path = Some(path);
                }
                Err(e)
              }
            },
          }
        }

        /// Transition the memory map to be readable and executable.
        /// If the memory map is file-backed, the file must have been opened with execute permissions.
        ///
        /// # Notes
        /// If `remove_on_drop` is set to `true`, then the underlying file will not be removed on drop if this function is invoked. [Read more]
        ///
        /// # Errors
        /// This method returns an error when the underlying system call fails,
        /// which can happen for a variety of reasons,
        /// such as when the file has not been opened with execute permissions
        ///
        /// # Examples
        ///
        #[doc = "```ignore"]
        #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
        /// # use scopeguard::defer;
        ///
        #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
        #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_freeze_exec_test.txt\").await.unwrap();")]
        #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_freeze_exec_test.txt\").unwrap());")]
        /// file.truncate(12).await;
        /// file.write_all("some data...".as_bytes(), 0).unwrap();
        /// file.flush().unwrap();
        /// // freeze_exec
        /// file.freeze_exec().unwrap();
        /// # })
        #[doc = "```"]
        ///
        #[doc = concat!("[Read more]: ", $path_str, "/struct.AsyncMmapFileMut.html#methods.set_remove_on_drop")]
        ///
        #[inline]
        pub fn freeze_exec(mut self) -> Result<AsyncMmapFile> {
          let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
          let inner = mem::replace(&mut self.inner, empty);
          let path = inner.path_buf();
          match inner {
            AsyncMmapFileMutInner::Empty(empty) => Ok(AsyncMmapFile::from(empty)), // unreachable, keep this for good measure
            AsyncMmapFileMutInner::Memory(memory) => Ok(AsyncMmapFile::from(memory.freeze())),
            AsyncMmapFileMutInner::Disk(disk) => match disk.freeze_exec() {
              Ok(frozen) => Ok(AsyncMmapFile::from(frozen)),
              Err(e) => {
                if !path.as_os_str().is_empty() {
                  self.pending_remove_path = Some(path);
                }
                Err(e)
              }
            },
          }
        }

        /// Returns whether remove the underlying file on drop.
        #[inline]
        pub fn get_remove_on_drop(&self) -> bool {
          self.remove_on_drop
        }

        /// Whether to remove the underlying file on drop. Default is false.
        ///
        /// # Notes
        /// If invoke [`AsyncMmapFileMut::freeze`], then the file will
        /// not be removed even though the field `remove_on_drop` is true.
        ///
        /// # Path-reuse safety
        ///
        /// `Drop` runs the same identity-checked, parent-bound unlink
        /// sequence as the explicit `remove()` / `drop_remove()` paths
        /// (POSIX: `fstatat` + `unlinkat` against a pre-opened parent
        /// dir fd, with the original async file's fd duped and held
        /// alive across the probe + unlink so the `(dev, ino)` we
        /// compare to can't be recycled). If the path no longer names
        /// the original inode — or if it became a symlink — `Drop`
        /// leaves it alone. The residual risks are the irreducible
        /// probe→unlink TOCTOU window and Windows' lack of true
        /// openat-equivalents; see `FileIdentity` for the full
        /// residual-race breakdown.
        ///
        /// If you require synchronous error reporting, call
        /// [`AsyncMmapFileMut::remove`] or
        /// [`AsyncMmapFileMut::drop_remove`] explicitly — `Drop`
        /// swallows errors because it cannot return a `Result`.
        ///
        /// [`AsyncMmapFileMut::freeze`]: structs.AsyncMmapFileMut.html#methods.freeze
        /// [`AsyncMmapFileMut::remove`]: structs.AsyncMmapFileMut.html#methods.remove
        /// [`AsyncMmapFileMut::drop_remove`]: structs.AsyncMmapFileMut.html#methods.drop_remove
        #[inline]
        pub fn set_remove_on_drop(&mut self, val: bool) {
          self.remove_on_drop = val;
        }

        /// Close the file. It would also truncate the file if max_sz >= 0.
        ///
        /// On error the wrapper keeps its original `Disk` inner (now
        /// poisoned), so the caller still has access to the path and can
        /// retry via `drop_remove` / `remove` / `Drop`. `Empty` is only
        /// installed after every fallible step succeeded.
        #[inline]
        pub async fn close(&mut self, max_sz: i64) -> Result<()> {
          if max_sz >= 0 && self.is_cow() {
            return Err(Error::new(
              ErrorKind::Unsupported,
              "cannot truncate a copy-on-write mmap file",
            ));
          }

          if max_sz >= 0 {
            // Run destructive work in-place on the disk inner. On Err the
            // disk is poisoned but still owns its path/file; the caller can
            // call `remove` / `drop_remove`.
            if let AsyncMmapFileMutInner::Disk(disk) = &mut self.inner {
              disk.close_with_truncate_in_place(max_sz as u64).await?;
            }
          } else {
            // No truncate — flush via dispatcher; on Err the inner is
            // unchanged.
            self.flush()?;
          }

          let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
          drop(mem::replace(&mut self.inner, empty));
          Ok(())
        }

        /// Remove the underlying file without dropping, leaving an `AsyncEmptyMmapFile`.
        #[inline]
        pub async fn remove(&mut self) -> Result<()> {
          // Retry the pending unlink from a prior failed `remove()` first
          // — path-reuse-safely (NeedsParentSync just syncs).
          if let Some(pending) = self.pending_drop_remove.take() {
            return match retry_pending_delete_async(pending).await {
              Ok(()) => {
                self.deleted = true;
                Ok(())
              }
              Err((pending, e)) => {
                self.pending_drop_remove = Some(pending);
                Err(e)
              }
            };
          }

          let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
          let inner = mem::replace(&mut self.inner, empty);
          match inner {
            #[allow(unused_mut)] // `mut` only needed on Unix smol-EMFILE recovery
            AsyncMmapFileMutInner::Disk(mut disk) => {
              let path = disk.path.clone();
              let identity = disk.file_identity;
              // Same runtime-specific helper as drop_remove. tokio
              // uses `into_std` (no fd alloc); smol falls back to dup
              // and on EMFILE returns the file so we can restore
              // self.inner — `remove(&mut self)` itself preserves self
              // for the caller to retry.
              #[cfg(unix)]
              let pin: std::fs::File = {
                let async_file = disk.file;
                match extract_pin_or_err(async_file).await {
                  Ok(p) => p,
                  Err((file_back, e)) => {
                    disk.file = file_back;
                    self.inner = AsyncMmapFileMutInner::Disk(disk);
                    return Err(e);
                  }
                }
              };
              #[cfg(not(unix))]
              drop(disk.file);
              drop(disk.mmap);
              match initial_remove_durably_async(
                &path,
                identity,
                #[cfg(unix)]
                pin,
              )
              .await
              {
                Ok(()) => {
                  self.deleted = true;
                  Ok(())
                }
                Err((pending, e)) => {
                  // Deletion was the user's explicit intent — record it so
                  // a subsequent `remove()` retry AND `Drop` (regardless of
                  // `remove_on_drop`) can re-attempt the unlink instead of
                  // leaking the file.
                  self.pending_drop_remove = Some(pending);
                  Err(e)
                }
              }
            }
            _ => {
              self.deleted = true;
              Ok(())
            }
          }
        }
      }

      impl_constructor_for_memory_mmap_file_mut!(AsyncMemoryMmapFileMut, AsyncMmapFileMut, "AsyncMmapFileMut", $path_str);

      impl_drop!(AsyncMmapFileMut, AsyncMmapFileMutInner, AsyncEmptyMmapFile);
    };
  }

  macro_rules! file_lock_tests {
    ($filename_prefix: literal, $runtime: meta) => {
      #[$runtime]
      async fn test_flush() {
        let path = concat!($filename_prefix, "_flush.txt");
        let mut file1 = AsyncMmapFileMut::create_with_options(path, AsyncOptions::new().max_size(100)).await.unwrap();
        file1.set_remove_on_drop(true);
        file1.write_all(vec![1; 100].as_slice(), 0).unwrap();
        file1.flush_range(0, 10).unwrap();
        file1.flush_async_range(11, 20).unwrap();
        file1.flush_async().unwrap();
      }

      #[$runtime]
      async fn test_lock_shared() {
        let path = concat!($filename_prefix, "_lock_shared.txt");
        let file1 = AsyncMmapFileMut::open(path).await.unwrap();
        let file2 = AsyncMmapFileMut::open(path).await.unwrap();
        let file3 = AsyncMmapFileMut::open(path).await.unwrap();
        defer!(let _ = std::fs::remove_file(path););

        // Concurrent shared access is OK, but not shared and exclusive.
        file1.lock_shared().unwrap();
        file2.lock_shared().unwrap();
        assert!(file3.try_lock().is_err());
        file1.unlock().unwrap();
        assert!(file3.try_lock().is_err());

        // Once all shared file locks are dropped, an exclusive lock may be created;
        file2.unlock().unwrap();
        file3.lock().unwrap();
      }
    };
  }
}

#[cfg(feature = "smol")]
#[cfg_attr(docsrs, doc(cfg(feature = "smol")))]
pub(crate) mod smol_impl;

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub(crate) mod tokio_impl;
