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
/// opened. Used by all deletion paths to verify that the path still names
/// the *same* inode before unlinking — otherwise a path-reused file
/// belonging to another actor could be silently deleted.
///
/// - POSIX: `(st_dev, st_ino)` from `MetadataExt`. Reliable: a (dev,ino)
///   pair uniquely identifies an inode while it exists.
/// - Windows: identity is **unavailable on stable Rust** at this crate's
///   MSRV (1.75). The natural identity (`GetFileInformationByHandle`'s
///   file index + volume serial) is exposed via
///   `std::os::windows::fs::MetadataExt::file_index` /
///   `volume_serial_number`, both gated behind the unstable
///   `windows_by_handle` feature (rust-lang/rust#63010) on stable
///   toolchains. We deliberately do NOT pull in `windows-sys` /
///   `winapi` to avoid a Windows-only build dep; instead we carry an
///   empty placeholder and let identity comparisons trivially succeed
///   on Windows. The path-reuse window between dropping the file
///   handle and `remove_file` is documented as a known limitation on
///   Windows.
#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
#[derive(Copy, Clone, Debug)]
pub(crate) struct FileIdentity {
  #[cfg(unix)]
  dev: u64,
  #[cfg(unix)]
  ino: u64,
  #[cfg(not(unix))]
  _placeholder: (),
}

#[cfg(any(feature = "sync", feature = "smol", feature = "tokio"))]
impl FileIdentity {
  /// Capture identity from a fresh metadata. The metadata MUST come from
  /// the open File handle (or its async equivalent), not from a path —
  /// path-derived metadata is racy with respect to renames.
  pub(crate) fn from_metadata(meta: &std::fs::Metadata) -> Self {
    #[cfg(unix)]
    {
      use std::os::unix::fs::MetadataExt;
      Self {
        dev: meta.dev(),
        ino: meta.ino(),
      }
    }
    #[cfg(not(unix))]
    {
      let _ = meta;
      Self { _placeholder: () }
    }
  }

  /// Strict equality for delete-safety checks. POSIX compares dev+ino.
  /// On non-POSIX (notably Windows) we have no platform-stable identity
  /// (see struct doc), so this returns `true` and lets the caller fall
  /// back to "path exists" semantics — accepting the documented
  /// path-reuse window as a known limitation.
  pub(crate) fn is_known_equal(&self, other: &Self) -> bool {
    #[cfg(unix)]
    {
      self.dev == other.dev && self.ino == other.ino
    }
    #[cfg(not(unix))]
    {
      let _ = other;
      true
    }
  }

  /// Returns `true` iff `path` currently names the same inode as this
  /// identity. POSIX: stat the path and compare dev+ino. Non-POSIX:
  /// the best we can do is confirm the path still resolves to *some*
  /// file — true identity verification needs Windows-specific APIs we
  /// don't carry (see struct doc).
  pub(crate) fn matches_path(&self, path: &Path) -> bool {
    #[cfg(unix)]
    {
      use std::os::unix::fs::MetadataExt;
      match std::fs::metadata(path) {
        Ok(m) => m.dev() == self.dev && m.ino() == self.ino,
        Err(_) => false,
      }
    }
    #[cfg(not(unix))]
    {
      std::fs::metadata(path).is_ok()
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
