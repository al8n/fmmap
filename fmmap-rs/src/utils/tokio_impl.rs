use tokio::fs::{File, OpenOptions};
use crate::error::{Error, Result};
#[cfg(feature = "nightly")]
use std::io;
use std::path::Path;

/// Sync directory
pub async fn sync_dir_async<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if !path.is_dir() {
        #[cfg(feature = "nightly")]
            return Err(Error::IO(io::Error::from(io::ErrorKind::NotADirectory)));
        #[cfg(not(feature = "nightly"))]
            return Err(Error::NotADirectory);
    }

    File::open(path)
        .await
        .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {}", path, e)))?
        .sync_all()
        .await
        .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {}", path, e)))
}

/// Open a read-only file
pub async fn open_read_only_file_async<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new()
        .read(true)
        .open(path)
        .await
        .map_err(Error::IO)
}

/// Open an existing file in write mode, all writes will overwrite the original file
pub async fn open_exist_file_async<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .append(false)
        .open(path)
        .await
        .map_err(Error::IO)
}

/// Open an existing file in write mode, all writes will append to the file
pub async fn open_exist_file_with_append_async<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .append(true)
        .open(path)
        .await
        .map_err(Error::IO)
}

/// Open an existing file and truncate it
pub async fn open_file_with_truncate_async<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .open(path)
        .await
        .map_err(Error::IO)
}

/// Open or create a file
pub async fn open_or_create_file_async<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
        .await
        .map_err(Error::IO)
}

/// Create a new file
pub async fn create_file_async<P: AsRef<Path>>(path: P) -> Result<File> {
    OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .append(true)
        .open(path)
        .await
        .map_err(Error::IO)
}