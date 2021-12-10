
use std::path::Path;
use crate::error::{Error, Result};

cfg_sync!(
    use std::fs::{File as SyncFile, OpenOptions as SyncOpenOptions};

    pub fn sync_dir<P: AsRef<Path>>(path: P) -> Result<()> {
        SyncFile::open(&path).map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {}", path.as_ref(), e)))?
            .sync_all()
            .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {}", path.as_ref(), e)))
    }

    /// Open a read-only file
    pub fn open_read_only_file<P: AsRef<Path>>(path: P) -> Result<SyncFile> {
        SyncOpenOptions::new()
        .read(true)
        .open(path)
    }

    /// Open an existing file in write mode, all writes will overwrite the original file
    pub fn open_exist_file<P: AsRef<Path>>(path: P) -> Result<SyncFile> {
        SyncOpenOptions::new()
        .read(true)
        .write(true)
        .append(false)
        .open(path)
    }

    /// Open an existing file in write mode, all writes will append to the file
    pub fn open_exist_file_with_append<P: AsRef<Path>>(path: P) -> Result<SyncFile> {
        SyncOpenOptions::new()
        .read(true)
        .write(true)
        .append(true)
        .open(path)
    }

    /// Open an existing file and truncate it
    pub fn open_file_with_truncate<P: AsRef<Path>>(path: P) -> Result<SyncFile> {
        SyncOpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .open(path)
    }

    /// Create a new file
    pub fn create_file<P: AsRef<Path>>(path: P) -> Result<SyncFile> {
        SyncOpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(path)
    }
);



cfg_tokio!(
    use tokio::fs::{File as TokioFile, OpenOptions as TokioOpenOptions};

    pub async fn sync_dir_async<P: AsRef<Path>>(path: P) -> Result<()> {
        TokioFile::open(&path).await.map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {}", path.as_ref(), e)))?
            .sync_all()
            .await
            .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {}", path.as_ref(), e)))
    }

        /// Open a read-only file
    pub fn open_read_only_file_async<P: AsRef<Path>>(path: P) -> Result<TokioFile> {
        TokioOpenOptions::new()
        .read(true)
        .open(path)
        .await
    }

    /// Open an existing file in write mode, all writes will overwrite the original file
    pub fn open_exist_file_async<P: AsRef<Path>>(path: P) -> Result<TokioFile> {
        TokioOpenOptions::new()
        .read(true)
        .write(true)
        .append(false)
        .open(path)
        .await
    }

    /// Open an existing file in write mode, all writes will append to the file
    pub fn open_exist_file_with_append_async<P: AsRef<Path>>(path: P) -> Result<TokioFile> {
        TokioOpenOptions::new()
        .read(true)
        .write(true)
        .append(true)
        .open(path)
        .await
    }

    /// Open an existing file and truncate it
    pub fn open_file_with_truncate_async<P: AsRef<Path>>(path: P) -> Result<TokioFile> {
        TokioOpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .open(path)
        .await
    }

    /// Create a new file
    pub fn create_file_async<P: AsRef<Path>>(path: P) -> Result<TokioFile> {
        TokioOpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(path)
        .await
    }
);