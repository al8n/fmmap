#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
use crate::error::{Error, ErrorKind, Result};
#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
use std::path::Path;

#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
fn not_a_directory_error() -> Error {
  Error::other("not a directory")
}

#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
fn no_parent_error() -> Error {
  Error::new(ErrorKind::InvalidInput, "path has no parent directory")
}

/// Sync a directory's metadata.
///
/// On POSIX this opens the directory and `fsync`s it — the standard way to
/// commit a metadata change (file creation, `set_len`, rename, etc.) to disk.
///
/// On Windows there's no equivalent operation: `FlushFileBuffers` on a
/// directory handle returns `ERROR_INVALID_FUNCTION`, because NTFS journals
/// metadata transactions through the *file's* `FlushFileBuffers` (already
/// performed by `file.sync_all()` upstream of this call). We still open the
/// directory — with `FILE_FLAG_BACKUP_SEMANTICS` and `access_mode(0)`, the
/// same combination `std::fs::canonicalize` uses — to surface real errors
/// (missing path, permission denied), then drop the handle without calling
/// `sync_all`, since the result is always a no-op error for dir handles.
#[cfg(all(windows, any(feature = "sync", feature = "smol", feature = "tokio")))]
pub(crate) fn sync_directory(path: &Path) -> Result<()> {
  use std::os::windows::fs::OpenOptionsExt;
  const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x02000000;
  if !path.is_dir() {
    return Err(not_a_directory_error());
  }
  // `access_mode(0)` requests no specific access — sufficient for opening
  // the dir handle and avoids needing FILE_LIST_DIRECTORY, which some
  // tightly-ACL'd parent dirs (e.g. user `Temp` on hardened CI runners)
  // don't grant.
  let _file = std::fs::OpenOptions::new()
    .access_mode(0)
    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
    .open(path)?;
  Ok(())
}

#[cfg(all(
  not(windows),
  any(feature = "sync", feature = "smol", feature = "tokio")
))]
pub(crate) fn sync_directory(path: &Path) -> Result<()> {
  if !path.is_dir() {
    return Err(not_a_directory_error());
  }
  std::fs::File::open(path)?.sync_all()
}

#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
fn sync_path_parent(path: &Path) -> Result<()> {
  let canonical = path.canonicalize()?;
  let parent = canonical.parent().ok_or_else(no_parent_error)?;
  sync_directory(parent)
}

/// Open the parent directory of `path` for later durability fsync.
///
/// Used by the durable-unlink paths to capture a stable handle to the
/// *original* parent inode *before* the unlink happens. After the
/// unlink, fsync'ing this handle is durable for the inode that actually
/// contained our entry, even if path resolution would now lead to a
/// different directory (parent rename / mount point swap between unlink
/// and fsync). Without this, a path-based parent fsync after the unlink
/// could fsync the wrong inode and report durable success.
///
/// On POSIX this opens with default flags (no O_RDONLY needed for fsync
/// on a dir handle, but `File::open` is what std exposes). On Windows we
/// use the same `access_mode(0)` / `FILE_FLAG_BACKUP_SEMANTICS` combo as
/// `sync_directory`, since neither read nor write is required and dir
/// handles need backup semantics.
///
/// Basename paths (no parent component) resolve to ".".
#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
pub(crate) fn open_parent_for_sync(path: &Path) -> Result<std::fs::File> {
  let parent_buf = match path.parent() {
    Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
    _ => std::path::PathBuf::from("."),
  };
  #[cfg(windows)]
  {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x02000000;
    std::fs::OpenOptions::new()
      .access_mode(0)
      .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
      .open(&parent_buf)
  }
  #[cfg(not(windows))]
  {
    std::fs::File::open(&parent_buf)
  }
}

/// Probe identity *relative to the open parent dir handle* on POSIX
/// using `fstatat`, falling back to path-based `from_path` on other
/// platforms.
///
/// Why parent-bound: mixing `open_parent_for_sync(path)`,
/// `metadata(path)`, `remove_file(path)`, and
/// `parent_handle.sync_all()` lets a parent-rename race make the
/// fsync claim durability for a different directory than the one the
/// unlink actually went through. Doing the probe AND the unlink (via
/// `unlink_at_or_path`) relative to the same `parent_handle` keeps all
/// three operations bound to the same parent inode.
///
/// On non-POSIX (Windows) we don't have a robust openat-equivalent
/// without major contortions, so we fall back to path-based — see the
/// Windows residual race documented in `FileIdentity`.
#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
pub(crate) fn identity_at_or_path(
  parent: &std::fs::File,
  full_path: &Path,
) -> Result<FileIdentity> {
  #[cfg(unix)]
  {
    use rustix::fs::{AtFlags, FileType};
    let basename = full_path.file_name().ok_or_else(|| {
      Error::new(
        ErrorKind::InvalidInput,
        "path has no basename (cannot statat)",
      )
    })?;
    let stat = rustix::fs::statat(parent, basename, AtFlags::SYMLINK_NOFOLLOW)
      .map_err(std::io::Error::from)?;
    if FileType::from_raw_mode(stat.st_mode as _) == FileType::Symlink {
      return Err(Error::other(format!(
        "identity-checked delete refuses to follow symlink at '{}': remove_file would unlink the symlink, not the target.",
        full_path.display(),
      )));
    }
    Ok(FileIdentity {
      dev: stat.st_dev as u64,
      ino: stat.st_ino as u64,
    })
  }
  #[cfg(not(unix))]
  {
    let _ = parent;
    FileIdentity::from_path(full_path)
  }
}

/// Unlink the file in a path-reuse-safe way, bound to a single
/// kernel-verified handle wherever possible.
///
/// **POSIX**: `unlinkat(parent_fd, basename, 0)`. Bound to the same
/// parent inode that `sync_parent_handle(parent)` will fsync — the
/// unlink is durable for the directory the entry was actually removed
/// from even if the parent path was renamed mid-operation. The
/// `expected_identity` param is unused here: the caller's
/// `identity_at_or_path` already verified, and there's no kernel API
/// to bind unlink to inode (vs name).
///
/// **Windows**: a path-based fallback (`std::fs::remove_file(path)`)
/// would have a TOCTOU between identity probe and delete — the probe
/// handle is closed, then `remove_file` re-opens the path, so a swap
/// in between could delete an unrelated replacement. This path
/// instead opens with `DELETE | FILE_SHARE_*` access and
/// `FILE_FLAG_OPEN_REPARSE_POINT`, re-verifies identity (and refuses
/// reparse points) on the handle, then issues
/// `SetFileInformationByHandle(FileDispositionInfo)`. The identity
/// check and the unlink are bound to the same handle — no race.
/// `expected_identity` is the value to verify against.
///
/// `parent` is unused on Windows (kept for cross-platform signature
/// parity).
#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
pub(crate) fn unlink_at_or_path(
  parent: &std::fs::File,
  full_path: &Path,
  expected_identity: FileIdentity,
) -> Result<()> {
  #[cfg(unix)]
  {
    let _ = expected_identity;
    use rustix::fs::AtFlags;
    let basename = full_path.file_name().ok_or_else(|| {
      Error::new(
        ErrorKind::InvalidInput,
        "path has no basename (cannot unlinkat)",
      )
    })?;
    rustix::fs::unlinkat(parent, basename, AtFlags::empty()).map_err(std::io::Error::from)?;
    Ok(())
  }
  #[cfg(windows)]
  {
    let _ = parent;
    use ::windows_sys::Win32::Storage::FileSystem::{
      FileBasicInfo, FileDispositionInfo, FileDispositionInfoEx, GetFileInformationByHandleEx,
      ReOpenFile, SetFileInformationByHandle, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY,
      FILE_BASIC_INFO, FILE_DISPOSITION_FLAG_DELETE,
      FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE, FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
      FILE_DISPOSITION_INFO, FILE_DISPOSITION_INFO_EX,
    };
    use std::os::windows::{
      fs::{MetadataExt, OpenOptionsExt},
      io::{AsRawHandle, FromRawHandle},
    };
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x02000000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x00200000;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x00000400;
    const DELETE_ACCESS: u32 = 0x00010000;
    const FILE_WRITE_ATTRIBUTES: u32 = 0x00000100;
    const FILE_SHARE_READ: u32 = 0x00000001;
    const FILE_SHARE_WRITE: u32 = 0x00000002;
    const FILE_SHARE_DELETE: u32 = 0x00000004;
    const INVALID_HANDLE_VALUE: isize = -1;
    // Open the primary delete handle with DELETE only.
    // FileDispositionInfoEx requires only DELETE access; requesting
    // FILE_WRITE_ATTRIBUTES upfront would fail at open for ACLs that
    // grant delete-only — and the Ex path's
    // IGNORE_READONLY_ATTRIBUTE flag means we never need
    // FILE_WRITE_ATTRIBUTES on this handle. The fallback re-opens
    // via ReOpenFile to add FILE_WRITE_ATTRIBUTES only when needed.
    // FILE_FLAG_OPEN_REPARSE_POINT means we get a handle to the
    // reparse entry itself if present, so the
    // FILE_ATTRIBUTE_REPARSE_POINT check below catches
    // symlinks/junctions before we delete them.
    let file = std::fs::OpenOptions::new()
      .access_mode(DELETE_ACCESS)
      .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
      .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
      .open(full_path)?;
    let meta = file.metadata()?;
    if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
      return Err(Error::other(format!(
        "identity-checked delete refuses to follow reparse point at '{}': remove would unlink the link/junction, not the target.",
        full_path.display(),
      )));
    }
    // Verify identity on the handle that will perform the delete
    // — this closes the probe→unlink TOCTOU.
    // SAFETY: file is alive for the duration of the call.
    let probe = unsafe { FileIdentity::from_raw_handle(file.as_raw_handle()) }?;
    if !expected_identity.is_known_equal(&probe) {
      return Err(Error::other(format!(
        "cannot unlink '{}': path no longer names the original file (path-reuse detected between handle drop and unlink)",
        full_path.display(),
      )));
    }
    // Try `FileDispositionInfoEx` first with `POSIX_SEMANTICS |
    // IGNORE_READONLY_ATTRIBUTE` so readonly files can still be
    // deleted on Windows 10 1607+.
    let handle_raw = file.as_raw_handle() as _;
    let info_ex = FILE_DISPOSITION_INFO_EX {
      Flags: FILE_DISPOSITION_FLAG_DELETE
        | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS
        | FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE,
    };
    let ok_ex = unsafe {
      SetFileInformationByHandle(
        handle_raw,
        FileDispositionInfoEx,
        &info_ex as *const _ as *const _,
        std::mem::size_of::<FILE_DISPOSITION_INFO_EX>() as u32,
      )
    };
    if ok_ex == 0 {
      // Legacy fallback for pre-1607 Windows / FAT32 etc.
      // FileDispositionInfo doesn't bypass readonly, so we explicitly
      // clear FILE_ATTRIBUTE_READONLY before issuing the legacy
      // delete. That requires FILE_WRITE_ATTRIBUTES, which the
      // primary handle doesn't have — re-open via `ReOpenFile` so the
      // access widening is bound to the same file (handle-based, not
      // path-based; no TOCTOU).
      // SAFETY: handle_raw is the live primary handle; ReOpenFile
      // returns a new HANDLE referring to the same underlying file.
      let reopen_raw = unsafe {
        ReOpenFile(
          handle_raw,
          DELETE_ACCESS | FILE_WRITE_ATTRIBUTES,
          FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
          FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
        )
      };
      if reopen_raw as isize == INVALID_HANDLE_VALUE {
        return Err(Error::last_os_error());
      }
      // SAFETY: we own reopen_raw exclusively now; wrapping as a
      // File ensures the handle is closed via Drop on every exit
      // path (success, error, panic).
      let reopened = unsafe { std::fs::File::from_raw_handle(reopen_raw as _) };
      let reopened_raw = reopened.as_raw_handle() as _;
      let mut basic: FILE_BASIC_INFO = unsafe { std::mem::zeroed() };
      let ok_get = unsafe {
        GetFileInformationByHandleEx(
          reopened_raw,
          FileBasicInfo,
          &mut basic as *mut _ as *mut _,
          std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
      };
      let was_readonly = ok_get != 0 && (basic.FileAttributes & FILE_ATTRIBUTE_READONLY) != 0;
      if was_readonly {
        let mut clear = basic;
        clear.FileAttributes &= !FILE_ATTRIBUTE_READONLY;
        // SetFileInformationByHandle treats `FileAttributes == 0`
        // as "do not change attributes"; if readonly was the only
        // bit set, our clear would be a no-op.
        // FILE_ATTRIBUTE_NORMAL (0x80) means "no other attributes
        // set" and IS a real attribute mutation, so use it as the
        // sentinel for the readonly-only case.
        if clear.FileAttributes == 0 {
          clear.FileAttributes = FILE_ATTRIBUTE_NORMAL;
        }
        // If clearing fails, still try disposition — surfaces the
        // disposition error, more relevant than the attribute-set
        // error.
        let _ = unsafe {
          SetFileInformationByHandle(
            reopened_raw,
            FileBasicInfo,
            &clear as *const _ as *const _,
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
          )
        };
      }
      let info = FILE_DISPOSITION_INFO { DeleteFile: true };
      let ok = unsafe {
        SetFileInformationByHandle(
          reopened_raw,
          FileDispositionInfo,
          &info as *const _ as *const _,
          std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
      };
      if ok == 0 {
        let err = Error::last_os_error();
        // Restore the readonly bit if disposition failed; the
        // file still exists on disk.
        if was_readonly {
          let _ = unsafe {
            SetFileInformationByHandle(
              reopened_raw,
              FileBasicInfo,
              &basic as *const _ as *const _,
              std::mem::size_of::<FILE_BASIC_INFO>() as u32,
            )
          };
        }
        return Err(err);
      }
      drop(reopened);
    }
    // Deletion happens on last close. `file` is our only handle
    // (FILE_SHARE_DELETE was set on open so other readers can hold
    // their handles); dropping it below triggers actual removal.
    drop(file);
    Ok(())
  }
  #[cfg(not(any(unix, windows)))]
  {
    let _ = (parent, expected_identity);
    std::fs::remove_file(full_path)
  }
}

/// Fsync a pre-opened parent directory handle.
///
/// On POSIX: `sync_all` commits the metadata change (our unlink) for the
/// inode this handle points at — durable even if the path it was opened
/// from has since been renamed.
/// On Windows: `FlushFileBuffers` on a directory handle returns
/// `ERROR_INVALID_FUNCTION`, so this is a no-op (NTFS journals dir
/// metadata via the file's `FlushFileBuffers`, which the caller has
/// already issued).
#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
pub(crate) fn sync_parent_handle(parent: &std::fs::File) -> Result<()> {
  #[cfg(unix)]
  {
    parent.sync_all()
  }
  #[cfg(windows)]
  {
    let _ = parent;
    Ok(())
  }
  #[cfg(not(any(unix, windows)))]
  {
    parent.sync_all()
  }
}

/// Platform identity of a file, captured once when the file is freshly
/// opened. Used by every deletion path as a *best-effort path-reuse
/// mitigation*.
///
/// # What this protects against
///
/// The broad window. Between the moment fmmap drops the original `File`
/// handle and the moment a retry / Drop tries to unlink, arbitrary time
/// can pass in which another actor may have removed the original file
/// and put a different one at the same path. The captured identity lets
/// every later unlink path probe the path, compare, and refuse on
/// mismatch — closing this *open-ended* race on both POSIX and Windows.
///
/// # What this does NOT protect against
///
/// **The narrow probe→unlink TOCTOU.** There is no atomic
/// "unlink-if-inode-matches" primitive in std. Between
/// `matches_path()` and the subsequent `std::fs::remove_file(path)` an
/// attacker with sufficient privileges to mutate the parent directory
/// could swap the entry. The window is one syscall (microseconds);
/// closing it fully would require platform-specific primitives such as
/// `unlinkat(parent_fd, basename, 0)` after `fstatat` (POSIX) or
/// `SetFileInformationByHandle(FileDispositionInfo)` on a handle
/// opened with `FILE_SHARE_DELETE` (Windows), neither of which std
/// exposes. We do pin the parent dir handle before unlink so the
/// post-unlink fsync is durable for the original parent inode, but the
/// unlink itself remains by-path.
///
/// # POSIX inode recycling
///
/// The kernel may reuse a recently-freed inode number. In theory a
/// fresh file at the same path can appear with the *same* `(st_dev,
/// st_ino)` as the original, defeating the comparison. In practice
/// this requires the original inode to have been fully released and
/// the kernel to reallocate that exact number — uncommon outside
/// small-id filesystems like tmpfs. Holding any handle on the original
/// inode (e.g. via the parent dir handle we keep around for fsync)
/// pins it and keeps the kernel from recycling its number.
///
/// # Platforms
///
/// - **POSIX**: `(st_dev, st_ino)` from `MetadataExt`. A `(dev,ino)` pair
///   uniquely identifies an inode while it exists.
/// - **Windows**: `(dwVolumeSerialNumber, nFileIndexHigh:nFileIndexLow)`
///   from `GetFileInformationByHandle`, via `windows-sys` —
///   `MetadataExt::file_index` would have required nightly's
///   `windows_by_handle` feature (rust-lang/rust#63010), so we go
///   straight to the Win32 API.
/// - Other targets compile to a placeholder; `is_known_equal` returns
///   `false` (refuse to identity-delete on platforms we can't verify).
#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileIdentity {
  #[cfg(unix)]
  dev: u64,
  #[cfg(unix)]
  ino: u64,
  #[cfg(windows)]
  volume_serial: u32,
  #[cfg(windows)]
  file_index: u64,
  #[cfg(not(any(unix, windows)))]
  _placeholder: (),
}

#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
impl FileIdentity {
  /// Capture identity from an open file handle. POSIX reads
  /// `Metadata`'s dev+ino; Windows calls `GetFileInformationByHandle`
  /// on the raw handle to get volume serial + file index.
  // Live on sync builds (disk/sync_impl.rs) and tests; async paths
  // use `from_raw_fd`/`from_raw_handle` directly on the borrowed
  // async file, so the std::fs::File overload is dead under
  // `--no-default-features --features tokio` / `--features smol`.
  #[allow(dead_code)]
  pub(crate) fn from_file(file: &std::fs::File) -> Result<Self> {
    #[cfg(unix)]
    {
      use std::os::unix::fs::MetadataExt;
      let m = file.metadata()?;
      Ok(Self {
        dev: m.dev(),
        ino: m.ino(),
      })
    }
    #[cfg(windows)]
    {
      use std::os::windows::io::AsRawHandle;
      // SAFETY: `info` is fully written by GetFileInformationByHandle
      // when it returns nonzero; we read it only on the success path.
      let mut info = ::std::mem::MaybeUninit::<
        ::windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION,
      >::uninit();
      let ok = unsafe {
        ::windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle(
          file.as_raw_handle() as ::windows_sys::Win32::Foundation::HANDLE,
          info.as_mut_ptr(),
        )
      };
      if ok == 0 {
        return Err(Error::last_os_error());
      }
      let info = unsafe { info.assume_init() };
      let file_index = ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64);
      Ok(Self {
        volume_serial: info.dwVolumeSerialNumber,
        file_index,
      })
    }
    #[cfg(not(any(unix, windows)))]
    {
      let _ = file;
      Ok(Self { _placeholder: () })
    }
  }

  /// Capture identity from a borrowed raw file descriptor (Unix).
  /// Used by async constructors whose runtime-specific File type isn't
  /// a `std::fs::File` but exposes `AsRawFd`. Avoids re-opening the
  /// path (which would race a between-handle-and-probe path swap).
  ///
  /// Identity capture is allocation-free. We `fstat` the borrowed fd
  /// directly — no `dup`, no descriptor allocated. A previous
  /// implementation applied a destructive `set_len` first and *then*
  /// dup'd; under fd pressure (EMFILE) the dup would fail and the
  /// user would get an error after their file had already been
  /// zeroed. No dup → no fd-allocation failure mode at identity
  /// capture time.
  ///
  /// # Safety
  /// `fd` must be a valid open file descriptor at call time, with no
  /// concurrent close.
  #[cfg(unix)]
  #[allow(dead_code)] // Only used by async features; sync-only builds skip this.
  pub(crate) unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Result<Self> {
    use std::os::fd::BorrowedFd;
    // SAFETY: caller guarantees `fd` is open at call time. We borrow
    // it only for the duration of `fstat`. No ownership transfer.
    let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
    let stat = rustix::fs::fstat(borrowed).map_err(std::io::Error::from)?;
    Ok(Self {
      dev: stat.st_dev as u64,
      ino: stat.st_ino as u64,
    })
  }

  /// Capture identity from a borrowed raw OS handle (Windows). See
  /// `from_raw_fd` for the rationale.
  ///
  /// Allocation-free — calls `GetFileInformationByHandle` directly
  /// on the borrowed handle. A previous implementation used
  /// `DuplicateHandle` + wrap-as-File + drop, which could fail under
  /// handle pressure after a destructive `set_len`.
  ///
  /// # Safety
  /// `handle` must be a valid open file HANDLE at call time, with no
  /// concurrent close.
  #[cfg(windows)]
  #[allow(dead_code)] // Only used by async features; sync-only builds skip this.
  pub(crate) unsafe fn from_raw_handle(handle: std::os::windows::io::RawHandle) -> Result<Self> {
    use ::windows_sys::Win32::Storage::FileSystem::{
      GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: caller guarantees `handle` is open at call time.
    // `GetFileInformationByHandle` reads through the handle without
    // taking ownership.
    let ok = unsafe { GetFileInformationByHandle(handle as _, &mut info) };
    if ok == 0 {
      return Err(Error::last_os_error());
    }
    Ok(Self {
      volume_serial: info.dwVolumeSerialNumber,
      file_index: ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
    })
  }

  /// Probe the path's identity without opening the file for I/O.
  ///
  /// **Symlinks / reparse points are refused.** `std::fs::remove_file`
  /// (Unix `unlink`, Windows `DeleteFile`) removes the directory entry
  /// at `path`, not its target — so even with a matching identity (the
  /// probe and the original handle both follow to the same target
  /// inode), unlinking would remove only the symlink/reparse entry
  /// while leaving the actual mapped file intact. Users who explicitly
  /// want to remove the link itself should use `std::fs::remove_file`
  /// directly.
  ///
  /// **Unix**: `symlink_metadata` (does not follow links) returns
  /// dev/ino directly. Stat-only — requires only execute permission on
  /// the parent dir, not read permission on the file (important so a
  /// `chmod 000` file in a writable parent is still
  /// identity-checkable).
  ///
  /// **Windows**: open the path with `FILE_FLAG_OPEN_REPARSE_POINT`
  /// (do not follow the link) and check `FILE_ATTRIBUTE_REPARSE_POINT`
  /// on the same handle's metadata. This is critical for correctness:
  /// a separate `symlink_metadata` pre-check followed by a
  /// reparse-following open is racy — an attacker can swap a regular
  /// file for a symlink between the two calls and the
  /// reparse-following open would happily resolve to the original
  /// target, making the identity match while `remove_file` later
  /// removes only the swapped-in link. The single-handle check binds
  /// "is this a reparse point?" to the same syscall as the identity
  /// probe.
  // On POSIX the wrappers use `identity_at_or_path` (parent-fd-bound
  // statat) instead, so this function is dead-code on Unix non-test
  // builds. Live on Windows and exotic platforms via the
  // `cfg(not(unix))` fallback in `identity_at_or_path` and the
  // `cfg(not(any(unix, windows)))` branches in `disk.rs`. Tests use it
  // on all platforms.
  #[allow(dead_code)]
  pub(crate) fn from_path(path: &Path) -> Result<Self> {
    #[cfg(unix)]
    {
      // `symlink_metadata` does NOT follow links; refuse symlinks so
      // a matching identity doesn't lead to `unlink` removing only
      // the link entry.
      let lmeta = std::fs::symlink_metadata(path)?;
      if lmeta.file_type().is_symlink() {
        return Err(Error::other(format!(
          "identity-checked delete refuses to follow symlink at '{}': remove_file would unlink the symlink, not the target. Use std::fs::remove_file or canonicalize the path yourself.",
          path.display(),
        )));
      }
      use std::os::unix::fs::MetadataExt;
      Ok(Self {
        dev: lmeta.dev(),
        ino: lmeta.ino(),
      })
    }
    #[cfg(windows)]
    {
      use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
      const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x02000000;
      const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x00200000;
      const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x00000400;
      // Open with FILE_FLAG_OPEN_REPARSE_POINT so we get a handle to
      // the reparse entry itself (if any) instead of the target. Then
      // refuse if FILE_ATTRIBUTE_REPARSE_POINT is set on the same
      // handle. Bound to one syscall — no TOCTOU between symlink check
      // and identity probe.
      let file = std::fs::OpenOptions::new()
        .access_mode(0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
      let meta = file.metadata()?;
      if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(Error::other(format!(
          "identity-checked delete refuses to follow reparse point at '{}': remove_file would unlink the link/junction, not the target.",
          path.display(),
        )));
      }
      Self::from_file(&file)
    }
    #[cfg(not(any(unix, windows)))]
    {
      let _ = path;
      Err(Error::other("FileIdentity unsupported on this platform"))
    }
  }

  /// Strict equality for delete-safety checks. Returns `false` on
  /// platforms without a platform-stable identity (which refuses
  /// identity-checked deletion rather than silently allowing it).
  pub(crate) fn is_known_equal(&self, other: &Self) -> bool {
    #[cfg(unix)]
    {
      self.dev == other.dev && self.ino == other.ino
    }
    #[cfg(windows)]
    {
      self.volume_serial == other.volume_serial && self.file_index == other.file_index
    }
    #[cfg(not(any(unix, windows)))]
    {
      let _ = other;
      false
    }
  }

  /// Returns `true` iff `path` currently names the same file as this
  /// identity. Returns `false` on any error (path missing, EACCES,
  /// etc.) — the conservative default is "do not delete".
  #[allow(dead_code)] // Test-only on Unix non-default builds.
  pub(crate) fn matches_path(&self, path: &Path) -> bool {
    match Self::from_path(path) {
      Ok(probe) => self.is_known_equal(&probe),
      Err(_) => false,
    }
  }
}

cfg_sync! {
  use std::fs::{File, OpenOptions};

  /// Sync directory metadata.
  ///
  /// On POSIX this opens the directory and calls `fsync` on it. On Windows
  /// the directory is opened with `FILE_FLAG_BACKUP_SEMANTICS` (so the open
  /// itself succeeds and surfaces real errors like missing path / permission
  /// denied), and the subsequent `FlushFileBuffers` returning
  /// `ERROR_INVALID_FUNCTION` — Windows can't flush dir handles — is silently
  /// treated as success because NTFS already journaled the metadata
  /// transaction when the file's `sync_all` ran.
  pub fn sync_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    sync_directory(path.as_ref())
  }

  /// Sync the parent directory of `path`. See [`sync_dir`] for the Windows
  /// rationale.
  pub fn sync_parent<P: AsRef<Path>>(path: P) -> Result<()> {
    sync_path_parent(path.as_ref())
  }

  /// Open a read-only file
  pub fn open_read_only_file<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new().read(true).open(path)
  }

  /// Open an existing file in write mode, all writes will overwrite the original file
  pub fn open_exist_file<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new()
      .read(true)
      .write(true)
      .append(false)
      .open(path)
  }

  /// Open an existing file in write mode, all writes will append to the file
  pub fn open_exist_file_with_append<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new().read(true).append(true).open(path)
  }

  /// Open an existing file and truncate it
  pub fn open_file_with_truncate<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new()
      .read(true)
      .write(true)
      .truncate(true)
      .open(path)
  }

  /// Open or create a file
  pub fn open_or_create_file<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new()
      .create(true)
      .truncate(false)
      .read(true)
      .write(true)
      .open(path)
  }

  /// Create a new file
  pub fn create_file<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new()
      .create_new(true)
      .read(true)
      .write(true)
      .open(path)
  }
}

cfg_async! {
  macro_rules! impl_async_file_utils {
    ($file: ident, $open_options: ident) => {
      /// Open a read-only file
      pub async fn open_read_only_file_async<P: AsRef<Path>>(path: P) -> Result<$file> {
        <$open_options>::new().read(true).open(path).await
      }

      /// Open an existing file in write mode, all writes will overwrite the original file
      pub async fn open_exist_file_async<P: AsRef<Path>>(path: P) -> Result<$file> {
        <$open_options>::new()
          .read(true)
          .write(true)
          .append(false)
          .open(path)
          .await
      }

      /// Open an existing file in write mode, all writes will append to the file
      pub async fn open_exist_file_with_append_async<P: AsRef<Path>>(path: P) -> Result<$file> {
        <$open_options>::new()
          .read(true)
          .write(true)
          .append(true)
          .open(path)
          .await
      }

      /// Open an existing file and truncate it
      pub async fn open_file_with_truncate_async<P: AsRef<Path>>(path: P) -> Result<$file> {
        <$open_options>::new()
          .read(true)
          .write(true)
          .truncate(true)
          .open(path)
          .await
      }

      /// Open or create a file
      pub async fn open_or_create_file_async<P: AsRef<Path>>(path: P) -> Result<$file> {
        <$open_options>::new()
          .create(true)
          .read(true)
          .write(true)
          .open(path)
          .await
      }

      /// Create a new file
      pub async fn create_file_async<P: AsRef<Path>>(path: P) -> Result<$file> {
        <$open_options>::new()
          .create_new(true)
          .read(true)
          .write(true)
          .open(path)
          .await
      }
    };
  }
}

cfg_smol! {
  /// file open utils for smol
  pub mod smol {
    use crate::error::Result;
    use smol::fs::{File, OpenOptions};
    use std::path::Path;

    /// Sync directory metadata. See the sync [`sync_dir`](super::sync_dir)
    /// for the Windows rationale.
    ///
    /// Implemented by dispatching to the blocking sync version via
    /// `smol::unblock` — the operation is brief and avoids duplicating the
    /// platform-specific dir-handle logic.
    pub async fn sync_dir_async<P: AsRef<Path>>(path: P) -> Result<()> {
      let path = path.as_ref().to_path_buf();
      ::smol::unblock(move || super::sync_directory(&path)).await
    }

    /// Sync the parent directory of `path`.
    pub async fn sync_parent_async<P: AsRef<Path>>(path: P) -> Result<()> {
      let path = path.as_ref().to_path_buf();
      ::smol::unblock(move || super::sync_path_parent(&path)).await
    }

    impl_async_file_utils!(File, OpenOptions);
  }
}

cfg_tokio! {
  /// file open utils for tokio
  pub mod tokio {
    use crate::error::Result;
    use std::path::Path;
    use tokio::fs::{File, OpenOptions};

    /// Sync directory metadata. See the sync [`sync_dir`](super::sync_dir)
    /// for the Windows rationale.
    ///
    /// Implemented by dispatching to the blocking sync version via
    /// `tokio::task::spawn_blocking` — the operation is brief and avoids
    /// duplicating the platform-specific dir-handle logic.
    pub async fn sync_dir_async<P: AsRef<Path>>(path: P) -> Result<()> {
      let path = path.as_ref().to_path_buf();
      ::tokio::task::spawn_blocking(move || super::sync_directory(&path))
        .await
        .map_err(std::io::Error::other)?
    }

    /// Sync the parent directory of `path`.
    pub async fn sync_parent_async<P: AsRef<Path>>(path: P) -> Result<()> {
      let path = path.as_ref().to_path_buf();
      ::tokio::task::spawn_blocking(move || super::sync_path_parent(&path))
        .await
        .map_err(std::io::Error::other)?
    }

    impl_async_file_utils!(File, OpenOptions);
  }
}
