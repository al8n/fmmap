use std::path::{Path, PathBuf};
#[cfg(not(target_os = "linux"))]
use std::ptr::{drop_in_place, write};
use async_trait::async_trait;
use crate::{MetaData, AsyncMmapFileExt, AsyncMmapFileMutExt};
use crate::disk::{MmapFileMutType, remmap};
use crate::error::Error;
use crate::options::AsyncOptions;
use crate::utils::{create_file_async, open_exist_file_with_append_async, open_read_only_file_async, sync_dir_async};
use memmap2::{Mmap, MmapMut, MmapOptions};
use tokio::fs::{File, remove_file};

/// AsyncDiskMmapFile contains an immutable mmap buffer
/// and a read-only file.
pub struct AsyncDiskMmapFile {
    pub(crate) mmap: Mmap,
    pub(crate) file: File,
    pub(crate) path: PathBuf,
    exec: bool,
}

impl_async_mmap_file_ext!(AsyncDiskMmapFile);

impl AsyncDiskMmapFile {
    /// Open a readable memory map backed by a file
    pub async fn open<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
        Self::open_in(path, None).await
    }

    /// Open a readable memory map backed by a file with [`Options`]
    ///
    /// [`Options`]: structs.Options.html
    pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
        Self::open_in(path, Some(opts)).await
    }

    /// Open a readable and executable memory map backed by a file
    pub async fn open_exec<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
        Self::open_exec_in(path, None).await
    }

    /// Open a readable and executable memory map backed by a file with [`Options`].
    ///
    /// [`Options`]: structs.Options.html
    pub async fn open_exec_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
        Self::open_exec_in(path, Some(opts)).await
    }

    async fn open_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
        let file = open_read_only_file_async(&path).await.map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;

        match opts  {
            None => {
                let mmap = unsafe {
                    Mmap::map(&file).map_err(|e| Error::MmapFailed(e.to_string()))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    exec: false
                })
            }
            Some(opts) => {
                let mmap = unsafe {
                    opts.mmap_opts.map(&file).map_err(|e| Error::MmapFailed(e.to_string()))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    exec: false,
                })
            }
        }
    }

    async fn open_exec_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
        let file = open_read_only_file_async(&path)
            .await
            .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;

        match opts  {
            None => {
                let mmap = unsafe {
                    MmapOptions::new().map_exec(&file).map_err(|e| Error::MmapFailed(e.to_string()))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    exec: true,
                })
            }
            Some(opts) => {
                let mmap = unsafe {
                    opts.mmap_opts.map_exec(&file).map_err(|e| Error::MmapFailed(e.to_string()))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    exec: true
                })
            }
        }
    }
}

/// AsyncDiskMmapFileMut contains a mutable mmap buffer
/// and a writable file.
pub struct AsyncDiskMmapFileMut {
    pub(crate) mmap: MmapMut,
    pub(crate) file: File,
    pub(crate) path: PathBuf,
    opts: Option<MmapOptions>,
    typ: MmapFileMutType,
}

impl_async_mmap_file_ext_for_mut!(AsyncDiskMmapFileMut);

#[async_trait]
impl AsyncMmapFileMutExt for AsyncDiskMmapFileMut {
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.mmap.as_mut()
    }

    fn is_cow(&self) -> bool {
        matches!(self.typ, MmapFileMutType::Cow)
    }

    impl_flush!();

    #[cfg(not(target_os = "linux"))]
    async fn truncate(&mut self, max_sz: u64) -> Result<(), Error> {
        // sync data
        self.flush()?;

        unsafe {
            // unmap
            drop_in_place(&mut self.mmap);

            // truncate
            self.file.set_len(max_sz).await.map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", self.path(), e)))?;

            // remap
            let mmap = remmap(self.path(), &self.file, self.opts.as_ref(), self.typ)?;

            write(&mut self.mmap, mmap);
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn truncate(&mut self, max_sz: u64) -> Result<(), Error> {
        // sync data
        self.flush()?;

        // truncate
        self.file.set_len(max_sz).await.map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", self.path(), e)))?;

        // remap
        self.mmap = remmap(self.path(), &self.file, self.opts.as_ref(), self.typ)?;

        Ok(())
    }

    async fn remove(mut self) -> crate::error::Result<()> {
        let path = self.path;
        drop(self.mmap);
        self.file.set_len(0).await.map_err(Error::IO)?;
        drop(self.file);
        remove_file(path).await.map_err(Error::IO)?;
        Ok(())
    }

    async fn close_with_truncate(self, max_sz: i64) -> crate::error::Result<()> {
        self.flush()?;
        drop(self.mmap);
        if max_sz >= 0 {
            self.file.set_len(max_sz as u64).await.map_err(Error::IO)?;
            let parent = self.path.parent().unwrap();
            sync_dir_async(parent).await?;
        }
        Ok(())
    }
}

impl AsyncDiskMmapFileMut {
    /// Create a new file and mmap this file
    pub async fn create<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::create_in(path, None).await
    }

    /// Create a new file and mmap this file with [`Options`]
    ///
    /// [`Options`]: structs.Options.html
    pub async fn create_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
        Self::create_in(path, Some(opts)).await
    }

    /// Open a file and mmap this file.
    /// The file will be open by [`File::open`].
    ///
    /// [`File::open`]: https://doc.rust-lang.org/std/fs/struct.File.html#method.open
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_in(path, None).await
    }

    /// Open a file and mmap this file with [`Options`].
    /// The file will be open by [OpenOptions::open]
    ///
    /// [`Options`]: structs.Options.html
    /// [`OpenOptions::open`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.open
    pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
        Self::open_in(path, Some(opts)).await
    }

    /// Open an existing file and mmap this file
    ///
    /// [`Options`]: structs.Options.html
    pub async fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_exist_in(path, None).await
    }

    /// Open an existing file and mmap this file with [`Options`]
    ///
    /// [`Options`]: structs.Options.html
    pub async fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
        Self::open_exist_in(path, Some(opts)).await
    }

    /// Open and mmap an existing file in copy-on-write mode.
    ///
    /// [`Options`]: structs.Options.html
    pub async fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_cow_in(path, None).await
    }

    /// Open and mmap an existing file in copy-on-write mode with [`Options`].
    /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file
    ///
    /// [`Options`]: structs.Options.html
    pub async fn open_cow_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
        Self::open_cow_in(path, Some(opts)).await
    }

    /// Returns an immutable version of this memory mapped buffer.
    /// If the memory map is file-backed, the file must have been opened with read permissions.
    ///
    /// # Errors
    /// This method returns an error when the underlying system call fails,
    /// which can happen for a variety of reasons,
    /// such as when the file has not been opened with read permissions.
    pub fn freeze(self) -> Result<AsyncDiskMmapFile, Error> {
        Ok(AsyncDiskMmapFile {
            mmap: self.mmap.make_read_only().map_err(Error::IO)?,
            file: self.file,
            path: self.path,
            exec: false,
        })
    }

    /// Transition the memory map to be readable and executable.
    /// If the memory map is file-backed, the file must have been opened with execute permissions.
    ///
    /// # Errors
    /// This method returns an error when the underlying system call fails,
    /// which can happen for a variety of reasons,
    /// such as when the file has not been opened with execute permissions
    pub fn freeze_exec(self) -> Result<AsyncDiskMmapFile, Error> {
        Ok(AsyncDiskMmapFile {
            mmap: self.mmap.make_exec().map_err(Error::IO)?,
            file: self.file,
            path: self.path,
            exec: true
        })
    }

    async fn create_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
        let file = create_file_async(&path)
            .await
            .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;

        match opts {
            None => {
                let mmap = unsafe { MmapMut::map_mut(&file).map_err(|e| Error::MmapFailed(e.to_string()))? };

                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: None,
                    typ: MmapFileMutType::Normal,
                })
            }
            Some(opts) => {
                if opts.max_size > 0 {
                    file.set_len(opts.max_size).await.map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", path.as_ref(), e)))?;
                    let parent = path.as_ref().parent().unwrap();
                    sync_dir_async(parent).await?;
                }

                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe { opts.mmap_opts.map_mut(&file).map_err(|e| Error::MmapFailed(e.to_string()))? };

                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: Some(opts_bk),
                    typ: MmapFileMutType::Normal,
                })
            }
        }
    }

    async fn open_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
        match opts {
            None => {
                let file = File::open(&path)
                    .await
                    .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;

                let mmap = unsafe { MmapMut::map_mut(&file).map_err(|e| Error::MmapFailed(e.to_string()))? };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: None,
                    typ: MmapFileMutType::Normal,
                })
            }
            Some(opts) => {
                let file = opts.file_opts.open(&path)
                    .await
                    .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;
                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe {
                    opts.mmap_opts.map_mut(&file).map_err(|e| Error::MmapFailed(e.to_string()))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: Some(opts_bk),
                    typ: MmapFileMutType::Normal,
                })
            }
        }
    }

    async fn open_exist_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
        let file = open_exist_file_with_append_async(&path)
            .await
            .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;

        match opts {
            None => {
                let mmap = unsafe { MmapMut::map_mut(&file)? };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: None,
                    typ: MmapFileMutType::Normal,
                })
            }
            Some(opts) => {
                let meta = file.metadata().await?;
                let file_sz = meta.len();
                if file_sz == 0 && opts.max_size > 0 {
                    file.set_len(opts.max_size).await.map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", path.as_ref(), e)))?;
                    let parent = path.as_ref().parent().unwrap();
                    sync_dir_async(parent).await?;
                }

                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe {
                    opts.mmap_opts.map_mut(&file)? };

                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: Some(opts_bk),
                    typ: MmapFileMutType::Normal,
                })
            }
        }
    }

    async fn open_cow_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
        let file = open_exist_file_with_append_async(&path)
            .await
            .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;

        match opts {
            None => {
                let mmap = unsafe { MmapOptions::new().map_copy(&file)? };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: None,
                    typ: MmapFileMutType::Cow,
                })
            }
            Some(opts) => {
                let meta = file.metadata().await?;
                let file_sz = meta.len();
                if file_sz == 0 && opts.max_size > 0 {
                    file.set_len(opts.max_size)
                        .await
                        .map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", path.as_ref(), e)))?;
                    let parent = path.as_ref().parent().unwrap();
                    sync_dir_async(parent).await?;
                }

                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe {
                    opts.mmap_opts.map_copy(&file)? };

                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: Some(opts_bk),
                    typ: MmapFileMutType::Cow,
                })
            }
        }
    }
}