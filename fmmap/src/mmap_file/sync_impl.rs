use crate::{
  disk::{DiskMmapFile, DiskMmapFileMut},
  empty::EmptyMmapFile,
  error::{Error, ErrorKind, Result},
  memory::{MemoryMmapFile, MemoryMmapFileMut},
  metadata::MetaData,
  options::Options,
  MmapFileReader, MmapFileWriter,
};
use std::{
  borrow::Cow,
  io::Cursor,
  mem,
  path::{Path, PathBuf},
};

/// fsync the directory `path` lives in so a freshly-`create_new`-opened
/// file becomes crash-durable by name (not just by content). Basename
/// paths get `.` so they're covered too.
fn sync_new_file_parent(path: &Path) -> Result<()> {
  let parent = match path.parent() {
    Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
    _ => std::path::PathBuf::from("."),
  };
  crate::utils::sync_dir(&parent)
}

/// Initial-call durable unlink with optional inode pin.
///
/// Sequence: open parent dir handle → probe identity at the path
/// (`fstatat` on POSIX, bound to the same parent fd) → unlink at the
/// path (`unlinkat` on POSIX, bound to the same parent fd) → fsync the
/// parent handle. All four ops are bound to the same parent inode on
/// POSIX, so a parent rename mid-operation can't direct durability to
/// the wrong directory.
///
/// `inode_pin` is the original `File` handle, held alive across probe +
/// unlink so the kernel cannot recycle the inode number while the
/// identity check runs. Required on POSIX where `(dev, ino)` is the
/// identity — without the pin, a concurrent replacement created right
/// after the caller closed the file could land on the same inode and
/// pass `is_known_equal`. On Windows, holding the handle without
/// `FILE_SHARE_DELETE` would *prevent* unlink, so the caller must drop
/// it before calling and pass `None`. The pin is dropped on function
/// exit, after the unlink.
///
/// Identity check distinguishes three cases: `NotFound` (original inode
/// presumed already gone, surface as `NeedsParentSync`); identity
/// mismatch (path was reused, refuse unlink, keep `NeedsUnlink`);
/// identity match (proceed with `unlinkat`).
fn initial_remove_durably(
  path: &Path,
  identity: crate::utils::FileIdentity,
  #[cfg(unix)] inode_pin: std::fs::File,
) -> std::result::Result<(), (crate::mmap_file::PendingDelete, Error)> {
  // pin is a required parameter on POSIX (not Option). Callers must
  // dup before calling and hard-fail on dup failure, so this function
  // never sees a missing pin. The pin is moved into
  // `PendingDelete::NeedsUnlink` on retryable failures so subsequent
  // retries inherit it; otherwise dropped on function exit.
  // Pin a handle to the *original* parent inode BEFORE the unlink. If
  // the path's parent gets renamed/replaced between unlink and fsync, a
  // path-based fsync would commit metadata on the wrong inode; the
  // pre-opened handle commits to the inode that actually contained our
  // directory entry. The handle is also stashed in any `NeedsParentSync`
  // pending state so retry fsyncs the same inode.
  let parent_handle = match crate::utils::open_parent_for_sync(path) {
    Ok(h) => h,
    Err(e) => {
      return Err((
        crate::mmap_file::PendingDelete::NeedsUnlink {
          path: path.to_path_buf(),
          identity,
          #[cfg(unix)]
          pin: inode_pin,
        },
        e,
      ));
    }
  };
  // Probe identity *relative to* parent_handle on POSIX (fstatat) so
  // the probe is bound to the same inode the unlink will go through —
  // a path-based probe could race a parent rename between
  // open_parent_for_sync and probe.
  match crate::utils::identity_at_or_path(&parent_handle, path) {
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
      // NotFound never proves crash-durable deletion. An nlink-based
      // "durably gone" inference would be wrong because an external
      // rename + unlink-elsewhere also produces `nlink == 0` while
      // leaving the actual unlink in a parent we don't fsync —
      // fsyncing OUR parent then doesn't make their unlink
      // crash-durable. Always route NotFound to NeedsUnlink so retry
      // surfaces NotFound rather than false-success-via-fsync.
      // Drop's NeedsUnlink path still best-effort fsyncs the parent,
      // which IS correct in the common "external rm in our parent"
      // case — we just don't promise the caller that deletion
      // succeeded.
      return Err((
        crate::mmap_file::PendingDelete::NeedsUnlink {
          path: path.to_path_buf(),
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
          path: path.to_path_buf(),
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
            path: path.to_path_buf(),
            identity,
            #[cfg(unix)]
            pin: inode_pin,
          },
          err,
        ));
      }
    }
  }
  // Bind the unlink to parent_handle (POSIX `unlinkat`) so the
  // subsequent `sync_parent_handle` is durable for the directory the
  // entry was actually removed from. There's still an irreducible
  // narrow TOCTOU between the identity probe and the unlinkat call,
  // but the *broad* race against parent renames is now closed.
  match crate::utils::unlink_at_or_path(&parent_handle, path, identity) {
    Ok(()) => match crate::utils::sync_parent_handle(&parent_handle) {
      Ok(()) => Ok(()),
      Err(e) => Err((
        crate::mmap_file::PendingDelete::NeedsParentSync {
          path: path.to_path_buf(),
          parent_handle,
        },
        e,
      )),
    },
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
      // Same as pre-probe NotFound — never claim durable deletion.
      // Post-probe NotFound from `unlinkat` could be a
      // rename-then-unlink-elsewhere; our parent fsync wouldn't
      // commit the right journal. Stay NeedsUnlink.
      Err((
        crate::mmap_file::PendingDelete::NeedsUnlink {
          path: path.to_path_buf(),
          identity,
          #[cfg(unix)]
          pin: inode_pin,
        },
        e,
      ))
    }
    Err(e) => Err((
      crate::mmap_file::PendingDelete::NeedsUnlink {
        path: path.to_path_buf(),
        identity,
        #[cfg(unix)]
        pin: inode_pin,
      },
      e,
    )),
  }
}

/// Retry a pending delete in a path-reuse-safe way.
///
/// `NeedsParentSync` only fsyncs the parent — never re-calls `remove_file`.
///
/// `NeedsUnlink { path, identity }` re-opens the path metadata, compares
/// against the captured identity, and only unlinks when they match. On
/// mismatch the path was reused, so we keep the pending state and return
/// a tagged error rather than deleting an unrelated file.
fn retry_pending_delete(
  pending: crate::mmap_file::PendingDelete,
) -> std::result::Result<(), (crate::mmap_file::PendingDelete, Error)> {
  match pending {
    crate::mmap_file::PendingDelete::NeedsParentSync {
      path,
      parent_handle,
    } => match crate::utils::sync_parent_handle(&parent_handle) {
      Ok(()) => Ok(()),
      Err(e) => Err((
        crate::mmap_file::PendingDelete::NeedsParentSync {
          path,
          parent_handle,
        },
        e,
      )),
    },
    crate::mmap_file::PendingDelete::NeedsUnlink {
      path,
      identity,
      #[cfg(unix)]
      pin,
    } => {
      // Just delegate to `initial_remove_durably`. It will re-stitch
      // the pin into `NeedsUnlink` on Err so the next retry inherits
      // a still-active inode pin.
      initial_remove_durably(
        &path,
        identity,
        #[cfg(unix)]
        pin,
      )
    }
  }
}

/// Utility methods to [`MmapFile`]
///
/// [`MmapFile`]: structs.MmapFile.html
#[enum_dispatch]
pub trait MmapFileExt {
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
  /// If there's not enough data, it would
  /// panic.
  fn slice(&self, offset: usize, sz: usize) -> &[u8] {
    &self.as_slice()[offset..offset + sz]
  }

  /// bytes returns data starting from offset off of size sz.
  ///
  /// # Errors
  /// If there's not enough data, it would return
  /// `Err(Error::from(ErrorKind::UnexpectedEof))`.
  fn bytes(&self, offset: usize, sz: usize) -> Result<&[u8]> {
    let buf = self.as_slice();
    super::checked_range(offset, sz, buf.len()).map(|range| &buf[range])
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

  /// Returns the metadata of file metadata
  ///
  /// Metadata information about a file.
  /// This structure is returned from the metadata or
  /// symlink_metadata function or method and represents
  /// known metadata about a file such as its permissions, size, modification times, etc
  fn metadata(&self) -> Result<MetaData>;

  /// Whether the mmap is executable.
  fn is_exec(&self) -> bool;

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
  /// — we deliberately do NOT mmap it. Mmapping a destination would push
  /// the crate's "no concurrent mutators / truncators" precondition onto
  /// every safe caller of this helper, which is a footgun: a caller in
  /// shared storage where another actor truncates the destination during
  /// the write would hit UB / SIGBUS.
  ///
  /// On success the new file is durably created: bytes synced via
  /// `sync_all`, and the parent directory fsynced so the new directory
  /// entry is committed too. Without the parent fsync, a crash could
  /// leave the data on disk but the filename absent.
  #[inline]
  fn write_all_to_new_file<P: AsRef<Path>>(&self, new_file_path: P) -> Result<()> {
    use std::io::Write as _;
    let path = new_file_path.as_ref();
    let buf = self.as_slice();
    let mut file = std::fs::OpenOptions::new()
      .create_new(true)
      .read(true)
      .write(true)
      .open(path)?;
    file.write_all(buf)?;
    file.sync_all()?;
    sync_new_file_parent(path)
  }

  /// Write a range of content of the mmap file to new file.
  #[inline]
  fn write_range_to_new_file<P: AsRef<Path>>(
    &self,
    new_file_path: P,
    offset: usize,
    len: usize,
  ) -> Result<()> {
    use std::io::Write as _;
    let path = new_file_path.as_ref();
    let buf = self.as_slice();
    let range = super::checked_range(offset, len, buf.len())?;
    // See `write_all_to_new_file` for the no-mmap rationale.
    let mut file = std::fs::OpenOptions::new()
      .create_new(true)
      .read(true)
      .write(true)
      .open(path)?;
    file.write_all(&buf[range])?;
    file.sync_all()?;
    sync_new_file_parent(path)
  }

  /// Returns a [`MmapFileReader`] which helps read data from mmap like a normal File.
  ///
  /// # Errors
  /// If there's not enough data, it would return
  ///  `Err(Error::from(ErrorKind::UnexpectedEof))`.
  ///
  /// [`MmapFileReader`]: structs.MmapFileReader.html
  fn reader(&self, offset: usize) -> Result<MmapFileReader<'_>> {
    let buf = self.as_slice();
    if buf.len() < offset {
      Err(Error::from(ErrorKind::UnexpectedEof))
    } else {
      Ok(MmapFileReader::new(
        Cursor::new(&buf[offset..]),
        offset,
        buf.len() - offset,
      ))
    }
  }

  /// Returns a [`MmapFileReader`] base on the given `offset` and `len`, which helps read data from mmap like a normal File.
  ///
  /// # Errors
  /// If there's not enough data, it would return
  ///  `Err(Error::from(ErrorKind::UnexpectedEof))`.
  ///
  /// [`MmapFileReader`]: structs.MmapFileReader.html
  fn range_reader(&self, offset: usize, len: usize) -> Result<MmapFileReader<'_>> {
    let buf = self.as_slice();
    let range = super::checked_range(offset, len, buf.len())?;
    Ok(MmapFileReader::new(Cursor::new(&buf[range]), offset, len))
  }

  /// Locks the file for exclusively usage, blocking if the file is currently locked.
  ///
  /// # Notes
  /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
  fn lock(&mut self) -> Result<()>;

  /// Locks the file for shared usage, blocking if the file is currently locked exclusively.
  ///
  /// # Safety
  /// On a `MmapFileMut` the constructor auto-acquired an exclusive lock to
  /// guarantee that no other writable or read-only mapping of the same file
  /// can be opened. Calling `lock_shared` on `flock`-style platforms downgrades
  /// that exclusive lock to a shared lock, which then allows another process
  /// (or another `fmmap` handle in the same process) to open a read-only
  /// mapping of the same file. The resulting concurrent `&mut [u8]` from this
  /// writer and `&[u8]` from the reader alias the same bytes — which is
  /// undefined behavior.
  ///
  /// Callers must ensure no conflicting mapping of the same file can be
  /// created for as long as this mapping (and any borrowed slices it has
  /// yielded) lives.
  ///
  /// On a read-only `MmapFile` this call is a no-op (the auto lock is already
  /// shared) and is sound, but is still marked `unsafe` because the trait is
  /// shared between read-only and writable types.
  ///
  /// # Notes
  /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
  unsafe fn lock_shared(&mut self) -> Result<()>;

  /// Locks the file for exclusively usage, or returns a an error if the file is currently locked (see lock_contended_error).
  ///
  /// # Notes
  /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
  fn try_lock(&mut self) -> Result<()>;

  /// Locks the file for shared usage, or returns a an error if the file is currently locked exclusively (see lock_contended_error).
  ///
  /// # Safety
  /// Same hazard as [`lock_shared`]: on a writable mapping this can downgrade
  /// the auto-acquired exclusive lock to a shared lock and allow another
  /// concurrent mapping of the same file, producing aliasing UB.
  ///
  /// # Notes
  /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
  unsafe fn try_lock_shared(&mut self) -> Result<()>;

  /// Unlocks the file.
  ///
  /// # Safety
  /// `MmapFile`/`MmapFileMut` constructors automatically acquire a file lock
  /// (shared or exclusive) to prevent the underlying file from being mapped
  /// concurrently with conflicting access. Calling `unlock` releases that
  /// guard; if any other process or `fmmap` instance subsequently opens the
  /// same file with a writable mapping while this mapping is alive, the two
  /// mappings will alias each other, which is undefined behavior.
  ///
  /// Callers must therefore ensure no conflicting mapping of the same file
  /// can be created for as long as this mapping (and any borrowed slices it
  /// has yielded) lives.
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

/// Utility methods to [`MmapFileMut`]
///
/// [`MmapFileMut`]: structs.MmapFileMut.html
#[enum_dispatch]
pub trait MmapFileMutExt {
  /// Returns the mutable underlying slice of the mmap
  fn as_mut_slice(&mut self) -> &mut [u8];

  /// slice_mut returns mutable data starting from offset off of size sz.
  ///
  /// # Panics
  /// If there's not enough data, it would
  /// panic.
  fn slice_mut(&mut self, offset: usize, sz: usize) -> &mut [u8] {
    &mut self.as_mut_slice()[offset..offset + sz]
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
    super::checked_range(offset, sz, buf.len()).map(|range| &mut buf[range])
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
  fn truncate(&mut self, max_sz: u64) -> Result<()>;

  /// Remove the underlying file
  fn drop_remove(self) -> Result<()>;

  /// Close and truncate the underlying file
  fn close_with_truncate(self, max_sz: i64) -> Result<()>;

  /// Returns a [`MmapFileWriter`] base on the given `offset`, which helps read or write data from mmap like a normal File.
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
  /// [`flush`]: traits.MmapFileMutExt.html#methods.flush
  /// [`flush_range`]: traits.MmapFileMutExt.html#methods.flush_range
  /// [`flush_async`]: traits.MmapFileMutExt.html#methods.flush_async
  /// [`flush_async_range`]: traits.MmapFileMutExt.html#methods.flush_async_range
  /// [`MmapFileWriter`]: structs.MmapFileWriter.html
  fn writer(&mut self, offset: usize) -> Result<MmapFileWriter<'_>> {
    let buf = self.as_mut_slice();
    let buf_len = buf.len();
    if buf_len < offset {
      Err(Error::from(ErrorKind::UnexpectedEof))
    } else {
      Ok(MmapFileWriter::new(
        Cursor::new(&mut buf[offset..]),
        offset,
        buf_len - offset,
      ))
    }
  }

  /// Returns a [`MmapFileWriter`] base on the given `offset` and `len`, which helps read or write data from mmap like a normal File.
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
  /// [`flush`]: traits.MmapFileMutExt.html#methods.flush
  /// [`flush_range`]: traits.MmapFileMutExt.html#methods.flush_range
  /// [`flush_async`]: traits.MmapFileMutExt.html#methods.flush_async
  /// [`flush_async_range`]: traits.MmapFileMutExt.html#methods.flush_async_range
  /// [`MmapFileWriter`]: structs.MmapFileWriter.html
  fn range_writer(&mut self, offset: usize, len: usize) -> Result<MmapFileWriter<'_>> {
    let buf = self.as_mut_slice();
    let range = super::checked_range(offset, len, buf.len())?;
    Ok(MmapFileWriter::new(
      Cursor::new(&mut buf[range]),
      offset,
      len,
    ))
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

#[enum_dispatch(MmapFileExt)]
enum MmapFileInner {
  Empty(EmptyMmapFile),
  Memory(MemoryMmapFile),
  Disk(DiskMmapFile),
}

/// A read-only memory map file.
///
/// There is 3 status of this struct:
/// - __Disk__: mmap to a real file
/// - __Memory__: use [`Bytes`] to mock a mmap, which is useful for test and in-memory storage engine
/// - __Empty__: a state represents null mmap, which is helpful for drop, close the `MmapFile`. This state cannot be constructed directly.
///
/// [`Bytes`]: https://docs.rs/bytes/1.1.0/bytes/struct.Bytes.html
#[repr(transparent)]
pub struct MmapFile {
  inner: MmapFileInner,
}

impl_mmap_file_ext!(MmapFile);

impl_from!(
  MmapFile,
  MmapFileInner,
  [EmptyMmapFile, MemoryMmapFile, DiskMmapFile]
);

impl MmapFile {
  /// Open a readable memory map backed by a file
  ///
  /// # Examples
  ///
  /// ```no_compile
  /// use fmmap::{MmapFile, MmapFileExt};
  /// use std::fs::{remove_file, File};
  /// use std::io::Write;
  /// # use scopeguard::defer;
  ///
  /// # let mut file = File::create("open_test.txt").unwrap();
  /// # defer!(remove_file("open_test.txt").unwrap());
  /// # file.write_all("some data...".as_bytes()).unwrap();
  /// # drop(file);
  ///
  /// // open and mmap the file
  /// let mut file = MmapFile::open("open_test.txt").unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  /// ```
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
    Ok(Self::from(DiskMmapFile::open(path)?))
  }

  /// Open a readable memory map backed by a file with [`Options`]
  ///
  /// # Examples
  ///
  /// ```no_compile
  /// use fmmap::{Options, MmapFile, MmapFileExt};
  /// # use scopeguard::defer;
  ///
  /// # let mut file = std::fs::File::create("open_test_with_options.txt").unwrap();
  /// # defer!(std::fs::remove_file("open_test_with_options.txt").unwrap());
  /// # std::io::Write::write_all(&mut file, "sanity text".as_bytes()).unwrap();
  /// # std::io::Write::write_all(&mut file, "some data...".as_bytes()).unwrap();
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
  /// let mut file = MmapFile::open_with_options("open_test_with_options.txt", opts).unwrap();
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
  pub unsafe fn open_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
    Ok(Self::from(DiskMmapFile::open_with_options(path, opts)?))
  }

  /// Open a readable memory map backed by a file
  ///
  /// # Examples
  ///
  /// ```no_compile
  /// use fmmap::{MmapFile, MmapFileExt};
  /// use std::fs::{remove_file, File};
  /// use std::io::Write;
  /// # use scopeguard::defer;
  ///
  /// # let mut file = File::create("open_exec_test.txt").unwrap();
  /// # defer!(remove_file("open_exec_test.txt").unwrap());
  /// # file.write_all("some data...".as_bytes()).unwrap();
  /// # drop(file);
  ///
  /// // open and mmap the file
  /// let mut file = MmapFile::open_exec("open_exec_test.txt").unwrap();
  /// let mut buf = vec![0; "some data...".len()];
  /// file.read_exact(buf.as_mut_slice(), 0);
  /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
  /// ```
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open_exec<P: AsRef<Path>>(path: P) -> Result<Self> {
    Ok(Self::from(DiskMmapFile::open_exec(path)?))
  }

  /// Open a readable and executable memory map backed by a file with [`Options`].
  ///
  /// # Examples
  ///
  /// ```no_compile
  /// use fmmap::{Options, MmapFile, MmapFileExt};
  /// # use scopeguard::defer;
  ///
  /// # let mut file = std::fs::File::create("open_exec_test_with_options.txt").unwrap();
  /// # defer!(std::fs::remove_file("open_exec_test_with_options.txt").unwrap());
  /// # std::io::Write::write_all(&mut file, "sanity text".as_bytes()).unwrap();
  /// # std::io::Write::write_all(&mut file, "some data...".as_bytes()).unwrap();
  /// # drop(file);
  ///
  /// // mmap the file with options
  /// let opts = Options::new()
  ///     // allow read
  ///     .read(true)
  ///     // mmap content after the sanity text
  ///     .offset("sanity text".as_bytes().len() as u64);
  /// // open and mmap the file
  /// let mut file = MmapFile::open_exec_with_options("open_exec_test_with_options.txt", opts).unwrap();
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
  pub unsafe fn open_exec_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
    Ok(Self::from(DiskMmapFile::open_exec_with_options(
      path, opts,
    )?))
  }
}

impl_constructor_for_memory_mmap_file!(MemoryMmapFile, MmapFile, "MmapFile", "sync");

#[enum_dispatch(MmapFileExt, MmapFileMutExt)]
enum MmapFileMutInner {
  Empty(EmptyMmapFile),
  Memory(MemoryMmapFileMut),
  Disk(DiskMmapFileMut),
}

/// A writable memory map file.
///
/// There is 3 status of this struct:
/// - __Disk__: mmap to a real file
/// - __Memory__: use [`BytesMut`] to mock a mmap, which is useful for test and in-memory storage engine
/// - __Empty__: a state represents null mmap, which is helpful for drop, remove, close the `MmapFileMut`. This state cannot be constructed directly.
///
/// [`BytesMut`]: https://docs.rs/bytes/1.1.0/bytes/struct.BytesMut.html
pub struct MmapFileMut {
  inner: MmapFileMutInner,
  remove_on_drop: bool,
  deleted: bool,
  /// User-requested deletion that failed and must be retried on `Drop`,
  /// regardless of `remove_on_drop`. The `PendingDelete` variant tracks
  /// whether the inode was already unlinked (so retry must NOT call
  /// `remove_file` again — path-reuse safety) or whether unlink itself
  /// still needs to happen.
  pending_drop_remove: Option<crate::mmap_file::PendingDelete>,
  /// Path retained so `Drop`'s opt-in `remove_on_drop` cleanup has a target
  /// after the inner mapping was already dropped — e.g. consuming
  /// `close_with_truncate(self)` failed mid-way and the inner is now
  /// `Empty`.
  pending_remove_path: Option<PathBuf>,
}

impl_from_mut!(
  MmapFileMut,
  MmapFileMutInner,
  [EmptyMmapFile, MemoryMmapFileMut, DiskMmapFileMut]
);

impl_mmap_file_ext!(MmapFileMut);

impl MmapFileMutExt for MmapFileMut {
  fn as_mut_slice(&mut self) -> &mut [u8] {
    self.inner.as_mut_slice()
  }

  fn is_cow(&self) -> bool {
    self.inner.is_cow()
  }

  impl_flush!();

  fn truncate(&mut self, max_sz: u64) -> Result<()> {
    // Just dispatch — the disk backend's `truncate` already keeps the
    // poisoned `DiskMmapFileMut` installed with its `path`/`file` intact,
    // and the disk-side `len` / `as_slice` / `as_mut_slice` accessors all
    // short-circuit to empty when `poisoned == true`. Swapping the inner
    // to `Empty` here would silently lose the path so `Drop`'s
    // `remove_on_drop` cleanup (and any subsequent `remove()` /
    // `drop_remove()` retry) couldn't find the (possibly-resized) file.
    self.inner.truncate(max_sz)
  }

  /// Remove the underlying file
  ///
  /// # Examples
  ///
  /// ```no_compile
  /// use fmmap::{MmapFileMut, MmapFileMutExt};
  /// # use scopeguard::defer;
  ///
  /// let mut file = MmapFileMut::create("remove_test.txt").unwrap();
  ///
  /// file.truncate(12);
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  ///
  /// file.drop_remove().unwrap();
  ///
  /// let err = std::fs::File::open("remove_test.txt");
  /// assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
  /// ```
  fn drop_remove(mut self) -> Result<()> {
    let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
    let inner = mem::replace(&mut self.inner, empty);
    match inner {
      MmapFileMutInner::Disk(disk) => {
        // Run the durable unlink at the wrapper layer so we can
        // classify failures correctly (`NeedsUnlink` vs
        // `NeedsParentSync`). If we delegated to the disk inner's
        // `drop_remove` instead, a parent-sync failure would be
        // indistinguishable from a real unlink failure, and Drop's
        // retry could call `remove_file` on a path that's already been
        // unlinked and possibly reused.
        let path = disk.path.clone();
        let identity = disk.file_identity;
        drop(disk.mmap);
        // Keep `disk.file` alive across the probe+unlink on POSIX so
        // the inode it refers to cannot be recycled (which would let
        // a fresh file at the same path pass identity). Windows:
        // holding the file without FILE_SHARE_DELETE would prevent
        // the unlink itself, so we drop it first and rely on
        // `(volume_serial, file_index)` not being recycled.
        #[cfg(unix)]
        let pin: std::fs::File = disk.file;
        #[cfg(not(unix))]
        drop(disk.file);
        match initial_remove_durably(
          &path,
          identity,
          #[cfg(unix)]
          pin,
        ) {
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
  /// ```no_compile
  /// use fmmap::{MetaDataExt, MmapFileMut, MmapFileExt, MmapFileMutExt};
  /// # use scopeguard::defer;
  ///
  /// let mut file = MmapFileMut::create("close_with_truncate_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("close_with_truncate_test.txt").unwrap());
  /// file.truncate(12);
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  ///
  /// file.close_with_truncate(50).unwrap();
  ///
  /// let file = MmapFileMut::open("close_with_truncate_test.txt").unwrap();
  /// let meta = file.metadata().unwrap();
  /// assert_eq!(meta.len(), 50);
  /// ```
  fn close_with_truncate(mut self, max_sz: i64) -> Result<()> {
    // COW mappings are private — by contract they must not mutate the
    // backing file. Refuse close-time truncation on COW; do it BEFORE
    // touching the inner so the original mapping stays usable on error.
    if max_sz >= 0 && self.is_cow() {
      return Err(Error::new(
        ErrorKind::Unsupported,
        "cannot truncate a copy-on-write mmap file",
      ));
    }

    // Capture the path now in case any in-place fallible step fails and
    // we need to surface a `pending_remove_path` for `remove_on_drop`.
    let path = self.inner.path_buf();

    if max_sz >= 0 {
      // Run the destructive work in-place so a transient flush/set_len/
      // sync failure leaves the disk inner *poisoned but intact* — its
      // file handle is preserved, matching the inherent `close()`'s
      // recovery model. Without this, a partial failure used to swap
      // inner with `Empty` and lose the path/file.
      if let MmapFileMutInner::Disk(disk) = &mut self.inner {
        if let Err(e) = disk.close_with_truncate_in_place(max_sz as u64) {
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

    // All fallible work succeeded. Now drop the disk inner.
    let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
    drop(mem::replace(&mut self.inner, empty));
    Ok(())
  }
}

impl MmapFileMut {
  /// Create a new file and mmap this file
  ///
  /// # Notes
  /// The new file is zero size, so before do write, you should truncate first.
  /// Or you can use [`Options::create_mmap_file_mut`] and set `max_size` field for [`Options`] to enable directly write
  /// without truncating.
  ///
  /// # Examples
  ///
  /// ```no_compile
  /// use fmmap::{Options, MmapFileMut, MmapFileMutExt, MmapFileExt};
  /// # use scopeguard::defer;
  ///
  /// let mut file = MmapFileMut::create("create_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("create_test.txt").unwrap());
  /// assert!(file.is_empty());
  /// assert_eq!(file.path_string(), String::from("create_test.txt"));
  ///
  /// file.truncate(12);
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  /// ```
  ///
  /// [`Options::create_mmap_file_mut`]: struct.Options.html#method.create_mmap_file_mut
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
    Ok(Self::from(DiskMmapFileMut::create(path)?))
  }

  /// Create a new file and mmap this file with [`Options`]
  ///
  /// # Examples
  ///
  /// ```no_compile
  /// use fmmap::{Options, MmapFileMut, MmapFileMutExt, MmapFileExt};
  /// # use scopeguard::defer;
  ///
  /// let opts = Options::new()
  ///     // truncate to 100
  ///     .max_size(100);
  /// let mut file = MmapFileMut::create_with_options("create_with_options_test.txt", opts).unwrap();
  /// # defer!(std::fs::remove_file("create_with_options_test.txt").unwrap());
  /// assert!(!file.is_empty());
  ///
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  /// ```
  ///
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn create_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
    Ok(Self::from(DiskMmapFileMut::create_with_options(
      path, opts,
    )?))
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
  /// ```no_compile
  /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt};
  /// use std::fs::File;
  /// use std::io::{Read, Write};
  /// # use scopeguard::defer;
  ///
  /// # let mut file = File::create("open_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("open_test.txt").unwrap());
  /// # file.write_all("some data...".as_bytes()).unwrap();
  /// # drop(file);
  ///
  /// // open and mmap the file
  /// let mut file = MmapFileMut::open("open_test.txt").unwrap();
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
  /// let mut file = File::open("open_test.txt").unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
  /// ```
  ///
  /// File does not exists
  ///
  /// ```no_run
  /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt};
  /// use std::fs::{remove_file, File};
  /// use std::io::{Read, Write};
  /// # use scopeguard::defer;
  ///
  /// // create and mmap the file
  /// let mut file = unsafe { MmapFileMut::open("open_test.txt") }.unwrap();
  /// # defer!(remove_file("open_test.txt").unwrap());
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
  /// let mut file = File::open("open_test.txt").unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
  /// ```
  ///
  /// [`open_with_options`]: struct.MmapFileMut.html#method.open_with_options
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
    Ok(Self::from(DiskMmapFileMut::open(path)?))
  }

  /// Open or Create(if not exists) a file and mmap this file with [`Options`].
  ///
  /// # Examples
  ///
  /// File already exists
  ///
  /// ```no_compile
  /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt, Options};
  /// use std::fs::{remove_file, File};
  /// use std::io::{Read, Seek, SeekFrom, Write};
  /// # use scopeguard::defer;
  ///
  /// # let mut file = File::create("open_test_with_options.txt").unwrap();
  /// # defer!(remove_file("open_test_with_options.txt").unwrap());
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
  /// let mut file = unsafe { MmapFileMut::open_with_options("open_test_with_options.txt", opts) }.unwrap();
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
  /// let mut file = File::open("open_test_with_options.txt").unwrap();
  /// file.seek(SeekFrom::Start("sanity text".as_bytes().len() as u64)).unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
  /// ```
  ///
  /// File does not exists
  ///
  /// ```no_run
  /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt, Options};
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
  /// let mut file = unsafe { MmapFileMut::open_with_options("open_test_with_options.txt", opts) }.unwrap();
  /// # defer!(std::fs::remove_file("open_test_with_options.txt").unwrap());
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
  /// let mut file = File::open("open_test_with_options.txt").unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
  /// ```
  ///
  /// [`Options`]: struct.Options.html
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
    Ok(Self::from(DiskMmapFileMut::open_with_options(path, opts)?))
  }

  /// Open an existing file and mmap this file
  ///
  /// # Examples
  /// ```no_compile
  /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt};
  /// use std::fs::File;
  /// use std::io::{Read, Write};
  /// # use scopeguard::defer;
  ///
  /// // create a temp file
  /// let mut file = File::create("open_existing_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("open_existing_test.txt").unwrap());
  /// file.write_all("some data...".as_bytes()).unwrap();
  /// drop(file);
  ///
  /// // mmap the file
  /// let mut file = MmapFileMut::open_exist("open_existing_test.txt").unwrap();
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
  /// let mut file = File::open("open_existing_test.txt").unwrap();
  /// file.read_exact(buf.as_mut_slice()).unwrap();
  /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
  /// ```
  /// # Safety
  ///
  /// See the [crate-level safety section](crate) for the full contract.
  ///
  pub unsafe fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self> {
    Ok(Self::from(DiskMmapFileMut::open_exist(path)?))
  }

  /// Open an existing file and mmap this file with [`Options`]
  ///
  /// # Examples
  /// ```no_compile
  /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt, Options};
  /// use std::fs::File;
  /// use std::io::{Read, Seek, SeekFrom, Write};
  /// # use scopeguard::defer;
  ///
  /// // create a temp file
  /// let mut file = File::create("open_existing_test_with_options.txt").unwrap();
  /// # defer!(std::fs::remove_file("open_existing_test_with_options.txt").unwrap());
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
  /// let mut file = MmapFileMut::open_exist_with_options("open_existing_test_with_options.txt", opts).unwrap();
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
  /// let mut file = File::open("open_existing_test_with_options.txt").unwrap();
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
  pub unsafe fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
    Ok(Self::from(DiskMmapFileMut::open_exist_with_options(
      path, opts,
    )?))
  }

  /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file).
  /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
  ///
  /// # Examples
  ///
  /// ```no_compile
  /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt};
  /// use std::fs::File;
  /// use std::io::{Read, Write};
  /// # use scopeguard::defer;
  ///
  /// // create a temp file
  /// let mut file = File::create("open_cow_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("open_cow_test.txt").unwrap());
  /// file.write_all("some data...".as_bytes()).unwrap();
  /// drop(file);
  ///
  /// // mmap the file
  /// let mut file = MmapFileMut::open_cow("open_cow_test.txt").unwrap();
  /// assert!(file.is_cow());
  ///
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
  /// let mut file = File::open("open_cow_test.txt").unwrap();
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
  pub unsafe fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self> {
    Ok(Self::from(DiskMmapFileMut::open_cow(path)?))
  }

  /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file) with [`Options`].
  /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
  ///
  /// # Examples
  ///
  /// ```no_compile
  /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt, Options};
  /// use std::fs::File;
  /// use std::io::{Read, Seek, Write, SeekFrom};
  /// # use scopeguard::defer;
  ///
  /// // create a temp file
  /// let mut file = File::create("open_cow_with_options_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("open_cow_with_options_test.txt").unwrap());
  /// file.write_all("sanity text".as_bytes()).unwrap();
  /// file.write_all("some data...".as_bytes()).unwrap();
  /// drop(file);
  ///
  /// // mmap the file with options
  /// let opts = Options::new()
  ///     // mmap content after the sanity text
  ///     .offset("sanity text".as_bytes().len() as u64);
  /// let mut file = MmapFileMut::open_cow_with_options("open_cow_with_options_test.txt", opts).unwrap();
  /// assert!(file.is_cow());
  ///
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
  /// let mut file = File::open("open_cow_with_options_test.txt").unwrap();
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
  pub unsafe fn open_cow_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
    Ok(Self::from(DiskMmapFileMut::open_cow_with_options(
      path, opts,
    )?))
  }

  /// Make the mmap file read-only.
  ///
  /// # Notes
  /// If `remove_on_drop` is set to `true`, then the underlying file will not be removed on drop if this function is invoked. [Read more]
  ///
  /// # Examples
  /// ```no_compile
  /// use fmmap::{MmapFileMut, MmapFileMutExt};
  /// # use scopeguard::defer;
  ///
  /// let mut file = MmapFileMut::create("mmap_file_freeze_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("mmap_file_freeze_test.txt").unwrap());
  /// file.truncate(12);
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  ///
  /// file.freeze().unwrap();
  /// ```
  ///
  /// [Read more]: structs.MmapFileMut.html#methods.set_remove_on_drop
  pub fn freeze(mut self) -> Result<MmapFile> {
    let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
    let inner = mem::replace(&mut self.inner, empty);
    let path = inner.path_buf();
    match inner {
      MmapFileMutInner::Empty(empty) => Ok(MmapFile::from(empty)), // unreachable, keep this for good measure
      MmapFileMutInner::Memory(memory) => Ok(MmapFile::from(memory.freeze())),
      MmapFileMutInner::Disk(disk) => match disk.freeze() {
        Ok(frozen) => Ok(MmapFile::from(frozen)),
        Err(e) => {
          // The disk is poisoned (or make_read_only failed). The wrapper
          // is being consumed; preserve the path so `Drop`'s opt-in
          // `remove_on_drop` cleanup can find the (possibly-mutated) file.
          // Not an explicit-delete request, so use `pending_remove_path`.
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
  /// # Errors
  /// This method returns an error when the underlying system call fails,
  /// which can happen for a variety of reasons,
  /// such as when the file has not been opened with execute permissions
  ///
  /// # Examples
  /// ```no_compile
  /// use fmmap::{MmapFileExt, MmapFileMut, MmapFileMutExt};
  /// # use scopeguard::defer;
  ///
  /// let mut file = MmapFileMut::create("mmap_file_freeze_exec_test.txt").unwrap();
  /// # defer!(std::fs::remove_file("mmap_file_freeze_exec_test.txt").unwrap());
  /// file.truncate(12);
  /// file.write_all("some data...".as_bytes(), 0).unwrap();
  /// file.flush().unwrap();
  ///
  /// let file = file.freeze_exec().unwrap();
  /// assert!(file.is_exec());
  /// ```
  pub fn freeze_exec(mut self) -> Result<MmapFile> {
    let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
    let inner = mem::replace(&mut self.inner, empty);
    let path = inner.path_buf();
    match inner {
      MmapFileMutInner::Empty(empty) => Ok(MmapFile::from(empty)), // unreachable, keep this for good measure
      MmapFileMutInner::Memory(memory) => Ok(MmapFile::from(memory.freeze())),
      MmapFileMutInner::Disk(disk) => match disk.freeze_exec() {
        Ok(frozen) => Ok(MmapFile::from(frozen)),
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
  /// If invoke [`MmapFileMut::freeze`], then the file will
  /// not be removed even though the field `remove_on_drop` is true.
  ///
  /// # Path-reuse safety
  ///
  /// `Drop` runs the same identity-checked, parent-bound unlink
  /// sequence as the explicit `remove()` / `drop_remove()` paths
  /// (POSIX: `fstatat` + `unlinkat` against a pre-opened parent dir
  /// fd, with the original file's fd duped and held alive across the
  /// probe + unlink so the `(dev, ino)` we compare to can't be
  /// recycled). If the path no longer names the original inode — or
  /// if it became a symlink — `Drop` leaves it alone. The residual
  /// risks are the irreducible probe→unlink TOCTOU window (one
  /// syscall) and Windows' lack of true openat-equivalents; see
  /// `FileIdentity` for the full residual-race breakdown.
  ///
  /// If you require synchronous error reporting, call
  /// [`MmapFileMut::remove`] or [`MmapFileMut::drop_remove`]
  /// explicitly before the wrapper is dropped — `Drop` swallows
  /// errors because it cannot return a `Result`.
  ///
  /// [`MmapFileMut::freeze`]: structs.MmapFileMut.html#methods.freeze
  /// [`MmapFileMut::remove`]: structs.MmapFileMut.html#methods.remove
  /// [`MmapFileMut::drop_remove`]: structs.MmapFileMut.html#methods.drop_remove
  #[inline]
  pub fn set_remove_on_drop(&mut self, val: bool) {
    self.remove_on_drop = val;
  }

  /// Close the file. It would also truncate the file if max_sz >= 0.
  ///
  /// On error the wrapper keeps its original `Disk` inner (now poisoned), so
  /// the caller still has access to the path and can retry via `drop_remove`
  /// / `remove` / `Drop`. `Empty` is only installed after every fallible step
  /// succeeded.
  #[inline]
  pub fn close(&mut self, max_sz: i64) -> Result<()> {
    // COW mappings are private — by contract they must not mutate the
    // backing file. Refuse close-time truncation on COW; do it BEFORE
    // touching the inner so the original mapping stays usable on error.
    if max_sz >= 0 && self.is_cow() {
      return Err(Error::new(
        ErrorKind::Unsupported,
        "cannot truncate a copy-on-write mmap file",
      ));
    }

    if max_sz >= 0 {
      // Run the destructive work in-place on the disk inner so a transient
      // flush/set_len/sync failure does NOT strand the wrapper with `Empty`
      // and lose the path. On Err the disk inner is poisoned but still owns
      // its `path` / `file`, so the caller can call `remove` / `drop_remove`.
      if let MmapFileMutInner::Disk(disk) = &mut self.inner {
        disk.close_with_truncate_in_place(max_sz as u64)?;
      }
      // Memory / Empty: nothing to do.
    } else {
      // No truncate requested — flush via the trait dispatcher; on Err the
      // inner is unchanged.
      self.flush()?;
    }

    // All fallible work succeeded. Now safe to drop the disk inner.
    let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
    drop(mem::replace(&mut self.inner, empty));
    Ok(())
  }

  /// Remove the underlying file without dropping, leaving an `EmptyMmapFile`.
  #[inline]
  pub fn remove(&mut self) -> Result<()> {
    // If a previous `remove()` call already dropped the inner mapping but
    // the unlink itself failed, retry that pending unlink first. Otherwise
    // a retry would short-circuit on the `_ => Ok(())` arm below (because
    // the inner is already Empty) and report success while the file still
    // exists.
    if let Some(pending) = self.pending_drop_remove.take() {
      return match retry_pending_delete(pending) {
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

    let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
    let inner = mem::replace(&mut self.inner, empty);
    match inner {
      MmapFileMutInner::Disk(disk) => {
        let path = disk.path;
        let identity = disk.file_identity;
        drop(disk.mmap);
        // Keep the inode pinned through probe+unlink on POSIX.
        // Windows must drop first (see drop_remove sibling).
        #[cfg(unix)]
        let pin: std::fs::File = disk.file;
        #[cfg(not(unix))]
        drop(disk.file);
        // Initial call: a missing file is the user's error. On other
        // failures we record `PendingDelete::NeedsUnlink`; if `remove_file`
        // itself succeeded but parent fsync didn't, we record
        // `NeedsParentSync` so retry doesn't re-call `remove_file` on a
        // possibly-reused path.
        match initial_remove_durably(
          &path,
          identity,
          #[cfg(unix)]
          pin,
        ) {
          Ok(()) => {
            self.deleted = true;
            Ok(())
          }
          Err((pending, e)) => {
            // Deletion was the user's explicit intent — record it so a
            // subsequent `remove()` retry AND `Drop` (regardless of
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

impl_constructor_for_memory_mmap_file_mut!(MemoryMmapFileMut, MmapFileMut, "MmapFileMut", "sync");

impl_drop!(MmapFileMut, MmapFileMutInner, EmptyMmapFile);

impl_sync_tests!("", MmapFile, MmapFileMut);

/// Extract the inode pin from a sync `std::fs::File` owned at Drop
/// time. Sync owns the file directly — just move it in. No fd
/// allocation, no EMFILE failure mode. Called by `impl_drop!`'s
/// `remove_on_drop` path; the macro's name resolution finds this
/// per-impl-file (sync here, async in `mmap_file/{tokio,smol}_impl.rs`).
#[cfg(unix)]
fn sync_drop_pin(file: std::fs::File) -> Option<std::fs::File> {
  Some(file)
}
#[cfg(not(unix))]
fn sync_drop_pin(file: std::fs::File) -> Option<std::fs::File> {
  drop(file);
  None
}

#[cfg(test)]
mod regression {
  use super::*;
  use crate::Options;
  use scopeguard::defer;
  use std::io::Write;

  /// Test helper: dup a `File`'s descriptor via `F_DUPFD_CLOEXEC` and
  /// wrap the new descriptor as an owned `File`. Mirrors the production
  /// dup that wrappers do before calling `initial_remove_durably`. Used
  /// to populate `PendingDelete::NeedsUnlink::pin` in tests that
  /// construct the pending state directly.
  #[cfg(unix)]
  fn dup_for_pin(file: &std::fs::File) -> std::fs::File {
    use std::os::fd::{AsRawFd, BorrowedFd};
    let raw = file.as_raw_fd();
    // SAFETY: `file` is a live, owned File; the borrow lives for the
    // call and `fcntl_dupfd_cloexec` returns a fresh OwnedFd.
    let borrowed = unsafe { BorrowedFd::borrow_raw(raw) };
    let owned = rustix::io::fcntl_dupfd_cloexec(borrowed, 0).expect("fcntl dup");
    std::fs::File::from(owned)
  }

  /// Finding #1: even though the public `lock_shared` is `unsafe`, calling it
  /// on a writable mapping does in fact downgrade the auto-acquired exclusive
  /// lock and thus allow another reader. This is documented as UB-on-the-caller;
  /// the test only confirms the unsafe contract is correctly described.
  #[test]
  fn auto_lock_blocks_aliased_writer_until_drop() {
    let path = "_regression_auto_lock_blocks.txt";
    defer!(let _ = std::fs::remove_file(path););
    let _ = std::fs::remove_file(path);
    let writer = unsafe { MmapFileMut::create(path) }.unwrap();
    // Second writer attempt on the same path must fail (exclusive lock held).
    assert!(unsafe { MmapFileMut::open(path) }.is_err());
    // Reader attempt must also fail (would conflict with exclusive).
    assert!(unsafe { MmapFile::open(path) }.is_err());
    drop(writer);
    // After the writer drops, both reader and writer can be opened (separately).
    let r = unsafe { MmapFile::open(path) }.unwrap();
    drop(r);
    let _ = unsafe { MmapFileMut::open(path) }.unwrap();
  }

  /// Finding #2: opening with `Options::truncate(true)` on a path whose lock
  /// is already held by another mapping must NOT destroy the existing file
  /// content. Truncation is now applied only after the auto-lock is acquired.
  #[test]
  fn lock_contended_open_with_truncate_preserves_content() {
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    // Pre-populate the file with content.
    {
      let mut f = std::fs::File::create(&path).unwrap();
      f.write_all(b"keep me").unwrap();
      f.sync_all().unwrap();
    }

    // First handle holds the exclusive lock.
    let holder = unsafe { MmapFileMut::open(&path) }.unwrap();

    // Second open with truncate(true) must fail at the lock step, NOT have
    // already truncated the file.
    let opts = Options::new().read(true).write(true).truncate(true);
    let err = match unsafe { MmapFileMut::open_with_options(&path, opts) } {
      Err(e) => e,
      Ok(_) => panic!("expected lock contention to fail the open"),
    };
    assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);

    // Drop the holder before reading: on Windows, an exclusive `LockFileEx`
    // hold blocks all access — even a plain `std::fs::read` from the same
    // process — so we must release first to verify content.
    drop(holder);
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, b"keep me");
  }

  /// A `truncate` failure that happens BEFORE the disk backend swaps
  /// in the anonymous placeholder (e.g. the cow-unsupported check)
  /// must NOT detach the live mapping. The original mapping should
  /// stay usable so the caller can flush/read it.
  #[test]
  fn pre_swap_truncate_failure_preserves_mapping() {
    let path = "_regression_pre_swap_truncate_preserves.txt";
    defer!(let _ = std::fs::remove_file(path););
    let _ = std::fs::remove_file(path);

    // Pre-populate with content.
    {
      let mut f = std::fs::File::create(path).unwrap();
      f.write_all(b"original").unwrap();
      f.sync_all().unwrap();
    }

    // Open in COW mode; truncate is unsupported on COW and returns Err
    // before touching the mapping.
    let mut cow = unsafe { MmapFileMut::open_cow(path) }.unwrap();
    let err = match cow.truncate(0) {
      Err(e) => e,
      Ok(()) => panic!("expected COW truncate to fail"),
    };
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);

    // The COW mapping must still be live: original bytes still readable.
    assert_eq!(cow.as_slice(), b"original");

    // Writes through the COW mapping still succeed (visible to this
    // handle only).
    cow.write_all(b"modified", 0).unwrap();
    assert_eq!(&cow.as_slice()[..b"modified".len()], b"modified");
  }

  /// Truncate clamp: opening with a large `len` and then truncating
  /// to a smaller size must NOT leave a mapping that extends past
  /// EOF. The crate clamps the stored `len` to `(new_size - offset)`
  /// on remap.
  #[test]
  fn truncate_clamps_oversized_len() {
    let path = "_regression_truncate_clamps_len.txt";
    defer!(let _ = std::fs::remove_file(path););
    let _ = std::fs::remove_file(path);

    // Pre-populate 8KiB.
    {
      let f = std::fs::File::create(path).unwrap();
      f.set_len(8192).unwrap();
      f.sync_all().unwrap();
    }

    // Open with explicit len = 8192.
    let opts = Options::new().read(true).write(true).len(8192);
    let mut file = unsafe { MmapFileMut::open_with_options(path, opts) }.unwrap();
    assert_eq!(file.len(), 8192);

    // Truncate to 1KiB. The new mapping must be 1024, not 8192.
    file.truncate(1024).unwrap();
    assert_eq!(file.len(), 1024);

    // Writes within the new bounds succeed.
    file.write_all(&[0xab; 1024], 0).unwrap();
    file.flush().unwrap();
  }

  /// Offset past EOF: truncating to below the mapping's offset must
  /// fail with InvalidInput rather than producing a broken mapping.
  /// The object remains usable (not poisoned) since we check before
  /// touching the mapping.
  #[test]
  fn truncate_below_offset_errors() {
    let path = "_regression_truncate_below_offset.txt";
    defer!(let _ = std::fs::remove_file(path););
    let _ = std::fs::remove_file(path);

    {
      let f = std::fs::File::create(path).unwrap();
      f.set_len(2048).unwrap();
      f.sync_all().unwrap();
    }

    let opts = Options::new().read(true).write(true).offset(1024);
    let mut file = unsafe { MmapFileMut::open_with_options(path, opts) }.unwrap();

    // truncate(500) would leave offset (1024) past the new EOF.
    let err = match file.truncate(500) {
      Err(e) => e,
      Ok(()) => panic!("expected InvalidInput"),
    };
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

    // The object is still usable: the mapping was never replaced because the
    // check fired before the placeholder swap.
    file.flush().unwrap();
  }

  /// `create_with_options` used to drop the user-set
  /// `Options::mode` / `custom_flags` because `create_in` opened with
  /// the hard-coded `create_file()` helper instead of routing through
  /// `opts.file_opts`. Verify on Unix that a custom mode is honored.
  #[cfg(unix)]
  #[test]
  fn create_with_options_honors_unix_mode() {
    use crate::Options;
    use std::os::unix::fs::PermissionsExt;

    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    let opts = Options::new().mode(0o600).max_size(8);
    let f = unsafe { MmapFileMut::create_with_options(&path, opts) }.unwrap();
    drop(f);

    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
  }

  /// `remove()` swaps the inner to Empty before calling `remove_file`.
  /// If the unlink fails, the original `MmapFileMut` is left with an
  /// Empty inner whose path is `""`, so a subsequent `Drop` can no
  /// longer attempt the unlink and the file leaks. Verify the
  /// wrapper retains the original path on failure (in
  /// `pending_drop_remove` because deletion was explicitly requested)
  /// so Drop has a chance to retry regardless of `remove_on_drop`.
  ///
  /// Unix-only: setup uses `std::fs::remove_file` which would fail
  /// against a still-open handle on Windows.
  #[cfg(unix)]
  #[test]
  fn remove_failure_retains_path_for_drop_retry() {
    let path = crate::tests::get_random_filename();
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();

    // Force the unlink to fail by pre-removing the file: the second
    // remove will fail with NotFound, so pending_drop_remove should be
    // populated.
    drop(std::fs::remove_file(&path));
    let err = f.remove().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    let pending = f.pending_drop_remove.as_ref().expect("pending state set");
    assert_eq!(pending.path(), path.as_path());
    // NotFound always routes to NeedsUnlink. An nlink-based
    // NeedsParentSync routing would assume the entry was unlinked
    // from OUR parent, but a rename + unlink-elsewhere also produces
    // nlink==0 — fsyncing our parent then doesn't make the other
    // directory's unlink crash-durable. Surfacing NeedsUnlink is
    // conservative: caller knows we did not confirm crash-durable
    // deletion.
    assert!(matches!(
      pending,
      crate::mmap_file::PendingDelete::NeedsUnlink { .. }
    ));
  }

  /// A subsequent `remove()` after a failed one used to
  /// short-circuit on the `_ => Ok(())` arm because the inner was
  /// Empty, reporting a successful cleanup while the original file
  /// still existed. Verify the retry actually attempts cleanup
  /// against the saved `pending_drop_remove`.
  ///
  /// NotFound at probe time stays NeedsUnlink. The retry re-probes;
  /// against a path-reused (recreated) file the identity check fails
  /// and we surface a path-reuse error. The recreated file is
  /// preserved.
  ///
  /// Unix-only: see sibling test for rationale.
  #[cfg(unix)]
  #[test]
  fn remove_retry_attempts_pending_path() {
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();

    // First call: pre-delete the file so remove() fails with NotFound.
    // Pending state is NeedsUnlink — we never confirmed the unlink
    // happened in OUR parent, so we can't claim crash-durable
    // deletion via parent fsync alone.
    drop(std::fs::remove_file(&path));
    assert!(f.remove().is_err());
    assert!(matches!(
      f.pending_drop_remove,
      Some(crate::mmap_file::PendingDelete::NeedsUnlink { .. })
    ));

    // Re-create a different file at the same path. The retry's
    // identity probe sees a different inode and surfaces a
    // path-reuse error; pending state stays NeedsUnlink. The
    // recreated file is preserved (we don't touch it).
    {
      let mut g = std::fs::File::create(&path).unwrap();
      use std::io::Write as _;
      g.write_all(b"different file").unwrap();
    }

    let err = f
      .remove()
      .expect_err("retry against path-reused file must NOT succeed");
    assert!(
      err.to_string().contains("path-reuse detected") || err.kind() == std::io::ErrorKind::NotFound,
      "expected path-reuse or NotFound, got: {err}"
    );
    assert!(path.exists(), "recreated file must remain");
    assert_eq!(std::fs::read(&path).unwrap(), b"different file");
    assert!(!f.deleted, "deleted must NOT be set on path-reuse retry");
  }

  /// When an explicit deletion (via `remove(&mut self)` or consuming
  /// `drop_remove(self)`) fails, an older implementation stored the
  /// retained path in `pending_remove_path`, which `Drop` only honors
  /// under `remove_on_drop=true` — and a caller asking for delete
  /// usually does NOT set that flag. Result: transient unlink
  /// failures silently leaked the file. Verify the
  /// `pending_drop_remove` channel triggers Drop's retry regardless
  /// of `remove_on_drop`.
  #[test]
  fn explicit_remove_failure_drop_retries_unconditionally() {
    let path = crate::tests::get_random_filename();
    {
      let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
      // Deliberately do NOT set remove_on_drop.
      f.truncate(8).unwrap();
      // Force remove() to fail with NotFound by pre-deleting the path.
      drop(std::fs::remove_file(&path));
      let err = f.remove().unwrap_err();
      assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
      // Recreate the file. The pending state is `NeedsParentSync` (NotFound
      // on initial → presumed already-unlinked), so Drop's retry only
      // fsyncs the parent and MUST NOT delete the recreated file (which
      // is unrelated to the original mapping — path-reuse safety).
      std::fs::File::create(&path).unwrap();
    }
    assert!(
      path.exists(),
      "Drop's path-reuse-safe retry must NOT delete a path-reused file",
    );
    let _ = std::fs::remove_file(&path);
  }

  /// The public `lock()` and `lock_shared()` methods used to call
  /// `fs4::FileExt::lock` / `lock_shared` blindly. POSIX `flock` is
  /// idempotent on the same handle, but Windows `LockFileEx` waits
  /// indefinitely for the same handle to release — deadlock. Verify
  /// that calling `lock()` / `lock_shared()` / `try_lock()` /
  /// `try_lock_shared()` on an auto-locked wrapper is reentrant-safe
  /// (no-op when state matches, `WouldBlock` when state mismatches).
  #[test]
  fn reentrant_lock_methods_do_not_deadlock_on_auto_lock() {
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    // Mut wrapper: auto-acquired exclusive lock.
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.lock()
      .expect("lock() on already-exclusive must be Ok no-op");
    f.try_lock()
      .expect("try_lock() on already-exclusive must be Ok no-op");
    let err = unsafe { f.lock_shared() }.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
    let err = unsafe { f.try_lock_shared() }.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
    drop(f);

    // Read-only wrapper: auto-acquired shared lock.
    let mut f = unsafe { MmapFile::open(&path) }.unwrap();
    unsafe { f.lock_shared() }.expect("lock_shared() on already-shared must be Ok no-op");
    unsafe { f.try_lock_shared() }.expect("try_lock_shared() on already-shared must be Ok no-op");
    let err = f.lock().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
    let err = f.try_lock().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);

    // After explicit unlock, lock() succeeds.
    unsafe { f.unlock() }.unwrap();
    f.lock().unwrap();
    unsafe { f.unlock() }.unwrap();
  }

  /// Durable unlink retry must be idempotent — after the first
  /// attempt unlinks the inode but fails `sync_dir`, the retry's
  /// parent fsync should still complete so the unlink isn't lost on
  /// crash. Verify `NeedsParentSync` retries fsync-only and reports
  /// success.
  ///
  /// We directly construct `NeedsParentSync` to exercise the
  /// post-unlink path. (A test that pre-deletes the path and relies
  /// on initial-call NotFound to produce `NeedsParentSync` does not
  /// work, because pre-unlink NotFound is correctly routed to
  /// `NeedsUnlink` — a renamed-away file isn't deleted, so that
  /// setup wouldn't reach the NeedsParentSync code path.)
  #[test]
  fn pending_drop_remove_retry_tolerates_already_unlinked() {
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();
    drop(std::fs::remove_file(&path));

    // Pre-open a parent handle to construct NeedsParentSync directly,
    // mirroring the post-unlink-pre-sync state initial_remove_durably
    // would have reached if `unlinkat` succeeded but
    // `sync_parent_handle` failed. The path is gone (we pre-deleted it
    // above) — the retry must still complete via parent fsync alone
    // and clear the pending state.
    let parent_handle = crate::utils::open_parent_for_sync(&path)
      .expect("open parent for NeedsParentSync test fixture");
    f.pending_drop_remove = Some(crate::mmap_file::PendingDelete::NeedsParentSync {
      path: path.clone(),
      parent_handle,
    });

    f.remove().unwrap();
    assert!(f.pending_drop_remove.is_none());
    assert!(f.deleted);
  }

  /// Pre-unlink NotFound after a *rename* (not an unlink) must keep
  /// `NeedsUnlink` and never mark `deleted = true`. The pin's inode
  /// is alive at the rename destination (`nlink > 0`), so the nlink
  /// classifier correctly stays in NeedsUnlink. A `NeedsParentSync`
  /// route would let the next retry trivially fsync and report
  /// success while the original file is still alive at a new name.
  #[cfg(unix)]
  #[test]
  fn pre_unlink_notfound_after_rename_keeps_needs_unlink() {
    let path = crate::tests::get_random_filename();
    let mut renamed = crate::tests::get_random_filename();
    renamed.set_extension("renamed");
    defer!(let _ = std::fs::remove_file(&path););
    defer!(let _ = std::fs::remove_file(&renamed););
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();

    // Rename the file away — directory entry at `path` is gone, but
    // the inode lives on at `renamed` (nlink stays at 1).
    std::fs::rename(&path, &renamed).unwrap();
    assert!(!path.exists());
    assert!(renamed.exists());

    // First remove() — pre-unlink probe at `path` is NotFound.
    // fstat on the pin sees nlink == 1 (alive at `renamed`), so we
    // route to NeedsUnlink (the rename-then-NotFound case).
    let err = f.remove().expect_err("path missing → NotFound");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert!(matches!(
      f.pending_drop_remove,
      Some(crate::mmap_file::PendingDelete::NeedsUnlink { .. })
    ));
    assert!(!f.deleted, "deleted must NOT be set on initial NotFound");

    // Retry — still missing at `path`, still alive at `renamed`,
    // still NeedsUnlink. No false-success.
    let err = f
      .remove()
      .expect_err("retry against renamed-away file stays NotFound");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert!(matches!(
      f.pending_drop_remove,
      Some(crate::mmap_file::PendingDelete::NeedsUnlink { .. })
    ));
    assert!(
      !f.deleted,
      "false-success on retry: the renamed-away file is still alive, deletion did not happen"
    );
    // The renamed-away file is still alive — we never touched it.
    assert!(renamed.exists());
  }

  /// `Drop` used to call `remove_file` on `inner.path_buf()` whenever
  /// `remove_on_drop=true`, regardless of inner variant. Memory
  /// variants store the user-supplied string as a label, so
  /// `MmapFileMut::memory_from_vec("real_file", ...)` followed by
  /// `set_remove_on_drop(true)` would delete `real_file` on Drop
  /// even though no on-disk mapping owns it. Verify Drop now no-ops
  /// on Memory variants (matching the explicit `remove()` method's
  /// behavior).
  #[test]
  fn drop_on_memory_variant_does_not_unlink_label_path() {
    let real_file_path = crate::tests::get_random_filename();
    {
      let mut g = std::fs::File::create(&real_file_path).unwrap();
      use std::io::Write as _;
      g.write_all(b"do not delete me").unwrap();
      g.sync_all().unwrap();
    }

    {
      let mut f = MmapFileMut::memory_from_vec(&real_file_path, vec![1, 2, 3]);
      f.set_remove_on_drop(true);
      // f drops here.
    }

    assert!(
      real_file_path.exists(),
      "Drop on a memory variant must not unlink a real file matching its label"
    );
    assert_eq!(std::fs::read(&real_file_path).unwrap(), b"do not delete me");
    let _ = std::fs::remove_file(&real_file_path);
  }

  /// `freeze`/`freeze_exec` on `DiskMmapFileMut` must check the
  /// `poisoned` flag — otherwise a failed truncate could be turned
  /// into a successful read-only `MmapFile` whose `as_slice()`
  /// returns the anonymous placeholder bytes while `path()`/
  /// `metadata()` refer to the real file — silently corrupt views.
  /// Verify `freeze` and `freeze_exec` reject a poisoned mapping.
  #[test]
  fn freeze_rejects_poisoned_mapping() {
    use crate::raw::DiskMmapFileMut;
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let opts = Options::new().read(true).write(true).max_size(64);
    let mut raw = unsafe { DiskMmapFileMut::create_with_options(&path, opts) }.unwrap();
    raw.force_poison_for_test();
    assert!(raw.is_poisoned());
    let err = raw.freeze().err().expect("freeze on poisoned should fail");
    assert!(
      err.to_string().contains("poisoned"),
      "expected poison error, got: {err}"
    );
  }

  #[test]
  fn freeze_exec_rejects_poisoned_mapping() {
    use crate::raw::DiskMmapFileMut;
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let opts = Options::new().read(true).write(true).max_size(64);
    let mut raw = unsafe { DiskMmapFileMut::create_with_options(&path, opts) }.unwrap();
    raw.force_poison_for_test();
    let err = raw
      .freeze_exec()
      .err()
      .expect("freeze_exec on poisoned should fail");
    assert!(
      err.to_string().contains("poisoned"),
      "expected poison error, got: {err}"
    );
  }

  /// Consuming `drop_remove(self)` used to swap inner to Empty BEFORE
  /// running fallible disk I/O. On Err the wrapper was consumed and
  /// `Drop`'s `remove_on_drop` cleanup silently no-op'd against the
  /// Empty inner's `""` path. Verify `drop_remove` propagates the
  /// Err (the `pending_remove_path` retention is verified by the
  /// inherent-`remove` regression above; the consuming variant uses
  /// the same retain-on-Err shape).
  #[test]
  fn drop_remove_consuming_propagates_failure() {
    let path = crate::tests::get_random_filename();
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();
    drop(std::fs::remove_file(&path));
    let err = f.drop_remove().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
  }
  /// inner to `Empty` before doing the fallible `flush`/`set_len`/`sync`
  /// work, so a transient I/O failure stranded the wrapper without a path
  /// or handle to retry/inspect. The fix runs the destructive work
  /// in-place on the disk inner (poisoning it on Err) and only swaps to
  /// Empty after success. Verify a successful close zeroes the file and
  /// installs Empty, and a no-truncate close (max_sz<0) is the same.
  #[test]
  fn close_with_truncate_in_place_succeeds() {
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();
    f.write_all(b"abcdefgh", 0).unwrap();
    f.flush().unwrap();
    f.close(4).unwrap();

    // After close: inner is Empty; methods route through the Empty arm.
    assert_eq!(MmapFileExt::len(&f), 0);
    assert_eq!(MmapFileExt::as_slice(&f), &[] as &[u8]);

    // Backing file is truncated to 4 bytes.
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, b"abcd");
  }

  /// A copy-on-write mapping must not mutate the backing file. Both
  /// `close(max_sz)` (inherent) and `close_with_truncate` (trait,
  /// dispatching to disk) used to call `set_len` on the backing file
  /// regardless of mapping type. Verify both paths refuse with
  /// Unsupported when `max_sz >= 0` AND the underlying file is
  /// preserved.
  #[test]
  fn cow_close_does_not_truncate_backing_file() {
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    {
      let mut f = std::fs::File::create(&path).unwrap();
      f.write_all(b"keep me intact").unwrap();
      f.sync_all().unwrap();
    }

    // inherent `MmapFileMut::close(max_sz >= 0)` on a COW mapping → Unsupported
    {
      let mut cow = unsafe { MmapFileMut::open_cow(&path) }.unwrap();
      let err = cow.close(0).unwrap_err();
      assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"keep me intact");

    // close(-1) is fine (no truncation) on COW
    {
      let mut cow = unsafe { MmapFileMut::open_cow(&path) }.unwrap();
      cow.close(-1).unwrap();
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"keep me intact");

    // trait `close_with_truncate(max_sz >= 0)` on a COW mapping → Unsupported
    {
      let cow = unsafe { MmapFileMut::open_cow(&path) }.unwrap();
      let err = cow.close_with_truncate(0).unwrap_err();
      assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"keep me intact");
  }

  /// `Options::len` was previously passed straight to memmapix
  /// without bounds-checking against the backing file. memmapix
  /// accepts the configured length unconditionally, so a 4096-byte
  /// mapping over a 1-byte file produces a mapping whose pages past
  /// EOF SIGBUS on access — turning a safe-API entry point into an
  /// unannounced UB trap. Verify each constructor rejects an
  /// `offset+len` window that exceeds the file before invoking
  /// memmapix.
  #[test]
  fn map_range_validation_rejects_oversized_window() {
    use crate::Options;
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    {
      let mut f = std::fs::File::create(&path).unwrap();
      f.write_all(b"abcd").unwrap(); // 4 bytes
    }

    let assert_invalid_input = |result: Result<()>| {
      let err = result.expect_err("expected InvalidInput rejection");
      assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    };

    // Mut create_with_options: file is brand new (0 bytes), no max_size,
    // but len=128. Constructor must reject before mmap.
    let create_path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&create_path););
    let opts = Options::new().len(128);
    assert_invalid_input(
      unsafe { MmapFileMut::create_with_options(&create_path, opts) }.map(|_| ()),
    );
    let _ = std::fs::remove_file(&create_path);

    // Mut open_with_options: existing 4-byte file, len=128.
    let opts = Options::new().len(128);
    assert_invalid_input(unsafe { MmapFileMut::open_with_options(&path, opts) }.map(|_| ()));

    // Mut open_exist_with_options: same.
    let opts = Options::new().len(128);
    assert_invalid_input(unsafe { MmapFileMut::open_exist_with_options(&path, opts) }.map(|_| ()));

    // COW open_cow_with_options: same.
    let opts = Options::new().len(128);
    assert_invalid_input(unsafe { MmapFileMut::open_cow_with_options(&path, opts) }.map(|_| ()));

    // Read-only open_with_options: same.
    let opts = Options::new().len(128);
    assert_invalid_input(unsafe { MmapFile::open_with_options(&path, opts) }.map(|_| ()));

    // offset past EOF (no len) is also rejected.
    let opts = Options::new().offset(64);
    assert_invalid_input(unsafe { MmapFile::open_with_options(&path, opts) }.map(|_| ()));

    // In-bounds window is accepted: len=2 at offset=1 fits in 4 bytes.
    let opts = Options::new().offset(1).len(2);
    let f = unsafe { MmapFile::open_with_options(&path, opts) }.unwrap();
    assert_eq!(f.as_slice(), b"bc");
  }

  /// The raw `DiskMmapFileMut::drop_remove` is consuming and used to
  /// return parent-fsync errors in a generic shape, so a caller
  /// couldn't tell unlink-failed from
  /// unlink-succeeded-but-parent-sync-failed and would be tempted to
  /// retry `remove_file` on a path that may have been reused. Verify
  /// the post-unlink failure is reported with a message that names
  /// the parent dir and warns against re-calling `remove_file`.
  ///
  /// Triggering an actual `sync_dir` failure is intrusive (mocking fsync),
  /// so this test only covers the success path of the message-tagging code:
  /// a successful drop_remove must not synthesize the warning. The error-
  /// path message construction is covered by code review and the matching
  /// async test below.
  #[test]
  fn raw_drop_remove_success_path_no_spurious_warning() {
    use crate::raw::DiskMmapFileMut;
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let mut f = unsafe { DiskMmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();
    f.flush().unwrap();
    f.drop_remove()
      .expect("drop_remove on a normal path must succeed");
    assert!(!path.exists(), "file should be unlinked");
  }

  /// `open_with_options` used to apply `truncate(true)` and
  /// `max_size` extension *before* validating the mapping range. An
  /// invalid `offset/len` combined with `truncate(true)` would zero
  /// an existing file and only then return Err — silent data loss.
  /// Verify the file content is preserved when validation rejects.
  #[test]
  fn invalid_options_with_truncate_preserve_existing_file() {
    use crate::Options;
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    {
      let mut f = std::fs::File::create(&path).unwrap();
      f.write_all(b"PRESERVE_ME").unwrap();
      f.sync_all().unwrap();
    }
    let original = std::fs::read(&path).unwrap();
    assert_eq!(original, b"PRESERVE_ME");

    // truncate(true) + offset past EOF (after planned truncate to 0).
    // Validation must reject before set_len(0) destroys the bytes.
    let opts = Options::new().truncate(true).offset(1).len(2);
    let result =
      unsafe { MmapFileMut::open_with_options(&path, opts) }.map(|_| "should have rejected");
    let err = result.expect_err("invalid offset/len must reject");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
      std::fs::read(&path).unwrap(),
      b"PRESERVE_ME",
      "file content must be intact after validation rejection"
    );

    // truncate(true) + max_size with len > max_size also rejects pre-truncate.
    let opts = Options::new().truncate(true).max_size(4).len(64);
    let result =
      unsafe { MmapFileMut::open_with_options(&path, opts) }.map(|_| "should have rejected");
    let err = result.expect_err("invalid offset/len must reject");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
      std::fs::read(&path).unwrap(),
      b"PRESERVE_ME",
      "file content must be intact after validation rejection"
    );
  }

  /// `pending_remove_path` (set when `close_with_truncate` consumed
  /// the inner) does NOT carry identity — by that point the `File`
  /// was already gone — so its Drop path is the path-reuse-safe
  /// "fsync parent only" behavior. Verify a wrapper whose only Drop
  /// signal is `pending_remove_path` does not unlink.
  #[test]
  fn pending_remove_path_drop_does_not_unlink() {
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    std::fs::write(&path, b"keep").unwrap();

    {
      let mut f = MmapFileMut::memory_from_vec("dummy.mem", vec![1u8]);
      f.pending_remove_path = Some(path.clone());
      f.remove_on_drop = true;
    }
    assert!(
      path.exists(),
      "pending_remove_path Drop path is identity-less and must not unlink",
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"keep");
  }

  /// `drop_complete_pending_delete` used to retry `remove_file` from
  /// `Drop` when state was `NeedsUnlink`, which races path reuse —
  /// between the initial failure and Drop, another actor could swap
  /// the path and our retry would delete an unrelated file. Verify
  /// Drop no longer unlinks by path alone.
  ///
  /// Construction: induce a `NeedsUnlink` pending state by
  /// pre-deleting the file to make the explicit `drop_remove()` fail
  /// with a typed non-NotFound error. (We can't easily provoke a
  /// non-NotFound failure in tests without privilege escalation;
  /// instead we directly install the pending state and observe Drop
  /// behavior.)
  ///
  /// Explicit `remove()` retry of a `NeedsUnlink` pending state must
  /// verify file identity before unlinking. If the path was reused
  /// between failure and retry, retry must NOT delete the unrelated
  /// occupant. POSIX uses dev+ino; Windows uses
  /// `GetFileInformationByHandle`'s volume serial + file index.
  #[test]
  fn explicit_retry_with_identity_check_refuses_path_reused_file() {
    let probe_path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&probe_path););

    // Capture identity from the *original* file, then simulate path
    // reuse. We must keep `original` open while we recreate the file:
    // on tmpfs (Linux CI) inode numbers are recycled aggressively, so
    // a `remove_file` + `write` sequence often hands back the same
    // (dev, ino), defeating the path-reuse simulation. Holding the
    // original handle pins its inode until end-of-test.
    let original = std::fs::File::create(&probe_path).unwrap();
    use std::io::Write;
    let mut original = original;
    original.write_all(b"original").unwrap();
    original.sync_all().unwrap();
    let original_identity = crate::utils::FileIdentity::from_file(&original).unwrap();
    // Unlink the directory entry but keep the inode pinned via `original`.
    std::fs::remove_file(&probe_path).unwrap();
    // Plant a *different* file at the same path. With `original` still
    // open, this gets a fresh inode.
    std::fs::write(&probe_path, b"unrelated content").unwrap();

    let mut f = MmapFileMut::memory_from_vec("dummy.mem", vec![1u8]);
    f.pending_drop_remove = Some(crate::mmap_file::PendingDelete::NeedsUnlink {
      path: probe_path.clone(),
      identity: original_identity,
      #[cfg(unix)]
      pin: dup_for_pin(&original),
    });

    let err = f.remove().unwrap_err();
    assert!(
      err.to_string().contains("path-reuse detected"),
      "expected path-reuse error, got: {err}",
    );
    assert!(
      probe_path.exists(),
      "retry must NOT have unlinked the path-reused file",
    );
    assert_eq!(std::fs::read(&probe_path).unwrap(), b"unrelated content");
    assert!(matches!(
      f.pending_drop_remove,
      Some(crate::mmap_file::PendingDelete::NeedsUnlink { .. })
    ));
    drop(original);
  }

  /// With a matching identity, retry succeeds and unlinks.
  #[test]
  fn explicit_retry_with_matching_identity_unlinks() {
    let probe_path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&probe_path););

    // Create the file and capture its identity. Don't delete it; the
    // path still names the same inode, so retry must succeed.
    let mut original = std::fs::File::create(&probe_path).unwrap();
    original.write_all(b"to-be-deleted").unwrap();
    original.sync_all().unwrap();
    let identity = crate::utils::FileIdentity::from_file(&original).unwrap();
    #[cfg(unix)]
    let pin = dup_for_pin(&original);
    drop(original);
    assert!(probe_path.exists());

    let mut f = MmapFileMut::memory_from_vec("dummy.mem", vec![1u8]);
    f.pending_drop_remove = Some(crate::mmap_file::PendingDelete::NeedsUnlink {
      path: probe_path.clone(),
      identity,
      #[cfg(unix)]
      pin,
    });

    f.remove().expect("identity matches → retry must unlink");
    assert!(!probe_path.exists(), "unlink must have succeeded");
    assert!(f.pending_drop_remove.is_none());
  }

  /// POSIX unlink permission is controlled by the parent dir, not by
  /// the file's mode bits — a `chmod 000` file in a writable parent
  /// must still be removable through fmmap's identity-checked path.
  /// A previous implementation opened the file to probe identity and
  /// so failed with EACCES on unreadable files; we now use
  /// `metadata()` (stat), which only needs execute on the parent dir.
  #[cfg(unix)]
  #[test]
  fn unix_identity_check_works_on_chmod_000_file() {
    use std::os::unix::fs::PermissionsExt;

    let probe_path = crate::tests::get_random_filename();
    defer!(let _ = {
      // Restore perms in case the test panics mid-way; otherwise the
      // tempfile cleanup helper can't remove the file.
      let _ = std::fs::set_permissions(&probe_path, std::fs::Permissions::from_mode(0o600));
      std::fs::remove_file(&probe_path)
    };);

    // Create the file, dup for the inode pin, capture identity.
    let original = std::fs::File::create(&probe_path).unwrap();
    let identity = crate::utils::FileIdentity::from_file(&original).unwrap();
    let pin = dup_for_pin(&original);
    drop(original);
    // Strip all permissions on the file. The parent dir (typically
    // /tmp with 0o1777 or similar) still permits unlink.
    std::fs::set_permissions(&probe_path, std::fs::Permissions::from_mode(0o000)).unwrap();

    // Identity probe via from_path uses metadata() now — works without
    // read permission.
    let probe = crate::utils::FileIdentity::from_path(&probe_path)
      .expect("metadata-based identity probe must succeed for chmod 000 file");
    assert!(identity.is_known_equal(&probe));
    assert!(identity.matches_path(&probe_path));

    // Wrap in PendingDelete and verify retry's identity check + unlink
    // succeeds (the parent dir is writable, the file is unreadable).
    let mut f = MmapFileMut::memory_from_vec("dummy.mem", vec![1u8]);
    f.pending_drop_remove = Some(crate::mmap_file::PendingDelete::NeedsUnlink {
      path: probe_path.clone(),
      identity,
      pin,
    });
    f.remove()
      .expect("chmod 000 file must still be removable via identity-checked retry");
    assert!(!probe_path.exists());
  }

  /// When the user passes a symlink path, `remove_file(path)`
  /// removes only the symlink, not the target — so even with
  /// matching identity (the probe and the open file both follow the
  /// symlink to the same inode), the wrapper would otherwise
  /// succeed-and-leave-the-real-file-behind. Verify that
  /// identity-checked cleanup refuses symlink paths instead.
  #[cfg(unix)]
  #[test]
  fn identity_check_refuses_symlink_path() {
    use std::os::unix::fs::symlink;

    let target_path = crate::tests::get_random_filename();
    let mut symlink_path = crate::tests::get_random_filename();
    symlink_path.set_extension("symlink");
    defer!(let _ = std::fs::remove_file(&target_path););
    defer!(let _ = std::fs::remove_file(&symlink_path););

    // Create the real file and a symlink pointing at it.
    {
      let f = std::fs::File::create(&target_path).unwrap();
      drop(f);
    }
    symlink(&target_path, &symlink_path).unwrap();

    // Capture identity through the symlink (follows). Both target and
    // symlink resolve to the same inode, so the matching-identity
    // happy path would otherwise green-light deletion.
    let original = std::fs::File::open(&symlink_path).unwrap();
    let identity = crate::utils::FileIdentity::from_file(&original).unwrap();
    let pin = dup_for_pin(&original);
    drop(original);
    // matches_path: symlink_metadata sees the link itself → refuse.
    assert!(
      !identity.matches_path(&symlink_path),
      "matches_path must refuse when path is a symlink",
    );

    // The from_path probe directly reports the symlink-refusal error.
    let err = crate::utils::FileIdentity::from_path(&symlink_path).unwrap_err();
    assert!(
      err.to_string().contains("refuses to follow symlink"),
      "expected symlink-refusal error, got: {err}",
    );

    // Plumbed through the wrapper: a NeedsUnlink retry against a
    // symlink path must NOT remove the symlink (or the target).
    let mut f = MmapFileMut::memory_from_vec("dummy.mem", vec![1u8]);
    f.pending_drop_remove = Some(crate::mmap_file::PendingDelete::NeedsUnlink {
      path: symlink_path.clone(),
      identity,
      pin,
    });
    let err = f.remove().unwrap_err();
    assert!(
      err.to_string().contains("refuses to follow symlink")
        || err.to_string().contains("path-reuse detected"),
      "expected symlink/path-reuse refusal, got: {err}",
    );
    assert!(symlink_path.exists(), "symlink must remain");
    assert!(target_path.exists(), "target must remain");
  }

  /// Drop's pending-delete path must verify identity before
  /// unlinking. With identity captured from the *original* file but
  /// the path now naming a different inode, Drop must leave it alone.
  #[test]
  fn drop_does_not_unlink_path_reused_file_for_needs_unlink() {
    let probe_path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&probe_path););

    // Hold the original file open across the path swap so its inode
    // is pinned (otherwise tmpfs may hand the same inode to the
    // replacement file and defeat the simulation; see sibling test).
    let original = std::fs::File::create(&probe_path).unwrap();
    use std::io::Write;
    let mut original = original;
    original.write_all(b"original").unwrap();
    original.sync_all().unwrap();
    let original_identity = crate::utils::FileIdentity::from_file(&original).unwrap();
    std::fs::remove_file(&probe_path).unwrap();
    std::fs::write(&probe_path, b"unrelated content").unwrap();

    {
      let mut f = MmapFileMut::memory_from_vec("dummy.mem", vec![1u8]);
      f.pending_drop_remove = Some(crate::mmap_file::PendingDelete::NeedsUnlink {
        path: probe_path.clone(),
        identity: original_identity,
        #[cfg(unix)]
        pin: dup_for_pin(&original),
      });
      drop(f);
    }

    assert!(
      probe_path.exists(),
      "Drop must NOT unlink a path-reused file (identity mismatch)",
    );
    assert_eq!(std::fs::read(&probe_path).unwrap(), b"unrelated content");
    drop(original);
  }

  /// Complement to the above — when identity matches, Drop's
  /// pending-delete path *does* unlink, now safely guarded by
  /// identity verification.
  #[test]
  fn drop_unlinks_when_identity_matches() {
    let probe_path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&probe_path););

    let mut original = std::fs::File::create(&probe_path).unwrap();
    original.write_all(b"to-be-deleted").unwrap();
    original.sync_all().unwrap();
    let identity = crate::utils::FileIdentity::from_file(&original).unwrap();
    #[cfg(unix)]
    let pin = dup_for_pin(&original);
    drop(original);
    assert!(probe_path.exists());

    {
      let mut f = MmapFileMut::memory_from_vec("dummy.mem", vec![1u8]);
      f.pending_drop_remove = Some(crate::mmap_file::PendingDelete::NeedsUnlink {
        path: probe_path.clone(),
        identity,
        #[cfg(unix)]
        pin,
      });
      drop(f);
    }

    assert!(
      !probe_path.exists(),
      "Drop must unlink when identity matches the path",
    );
  }

  /// The *initial* `remove()` / `drop_remove()` call must
  /// identity-check before `remove_file`. Between the wrapper
  /// dropping its file handle and the unlink, an outside actor could
  /// rename + recreate the path with a different file. Without the
  /// check, the initial unlink would delete that unrelated file.
  /// Simulate the race by:
  ///   1. opening the wrapper (captures identity from the original inode),
  ///   2. externally swapping the path with a different file,
  ///   3. calling `remove()`, which must refuse to unlink.
  #[test]
  fn initial_remove_refuses_path_reused_file() {
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();

    // Simulate the race: replace the file at the path with a different
    // file (different inode). `MmapFileMut`'s captured identity now
    // points at an inode that's been unlinked; the path names a new file.
    std::fs::remove_file(&path).unwrap();
    std::fs::write(&path, b"unrelated content").unwrap();

    let err = f.remove().unwrap_err();
    assert!(
      err.to_string().contains("path-reuse detected"),
      "expected path-reuse error, got: {err}",
    );
    assert!(
      path.exists(),
      "initial unlink must NOT have deleted the path-reused file",
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"unrelated content");
    // Pending state is `NeedsUnlink` so the user sees the unfinished
    // cleanup; retry would also refuse.
    assert!(matches!(
      f.pending_drop_remove,
      Some(crate::mmap_file::PendingDelete::NeedsUnlink { .. })
    ));
  }

  /// Same race as the wrapper test, but for raw
  /// `DiskMmapFileMut::drop_remove`. The raw API must not unlink
  /// either.
  #[test]
  fn raw_drop_remove_refuses_path_reused_file() {
    use crate::raw::DiskMmapFileMut;
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let mut f = unsafe { DiskMmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();
    f.flush().unwrap();

    // Race window: swap path content with a different file.
    std::fs::remove_file(&path).unwrap();
    std::fs::write(&path, b"unrelated content").unwrap();

    let err = f.drop_remove().unwrap_err();
    assert!(
      err.to_string().contains("path-reuse detected"),
      "expected path-reuse error, got: {err}",
    );
    assert!(
      path.exists(),
      "raw drop_remove must not delete a path-reused file"
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"unrelated content");
  }

  /// `remove_on_drop` direct path verifies identity from the inner
  /// before unlinking. Verify the file IS unlinked when the path
  /// still names the same inode at Drop time.
  #[test]
  fn remove_on_drop_unlinks_when_identity_matches() {
    let path = crate::tests::get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    {
      let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
      f.set_remove_on_drop(true);
    }
    assert!(
      !path.exists(),
      "remove_on_drop must unlink when path identity matches",
    );
  }
}
