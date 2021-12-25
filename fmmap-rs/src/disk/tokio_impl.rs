use std::path::{Path, PathBuf};
#[cfg(not(target_os = "linux"))]
use std::ptr::{drop_in_place, write};
use async_trait::async_trait;
use crate::{MetaData, AsyncMmapFileExt, AsyncMmapFileMutExt};
use crate::disk::{MmapFileMutType, remmap};
use crate::error::Error;
use crate::options::AsyncOptions;
use crate::utils::{create_file_async, open_exist_file_with_append_async, open_or_create_file_async, open_read_only_file_async, sync_dir_async};
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::AsyncMmapFileExt;
    /// use fmmap::raw::AsyncDiskMmapFile;
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_disk_open_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    /// // mmap the file
    /// let mut file = AsyncDiskMmapFile::open("async_disk_open_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    pub async fn open<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
        Self::open_in(path, None).await
    }

    /// Open a readable memory map backed by a file with [`Options`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{AsyncOptions, AsyncMmapFileExt};
    /// use fmmap::raw::AsyncDiskMmapFile;
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_disk_open_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_with_options_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    ///
    /// // mmap the file
    /// let mut opts = AsyncOptions::new();
    /// opts
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// // mmap the file
    /// let mut file = AsyncDiskMmapFile::open_with_options("async_disk_open_with_options_test.txt", opts).await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: structs.AsyncOptions.html
    pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
        Self::open_in(path, Some(opts)).await
    }

    /// Open a readable and executable memory map backed by a file
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::AsyncMmapFileExt;
    /// use fmmap::raw::AsyncDiskMmapFile;
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_disk_open_exec_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_exec_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    /// // mmap the file
    /// let mut file = AsyncDiskMmapFile::open_exec("async_disk_open_exec_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    pub async fn open_exec<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
        Self::open_exec_in(path, None).await
    }

    /// Open a readable and executable memory map backed by a file with [`Options`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{AsyncOptions, AsyncMmapFileExt};
    /// use fmmap::raw::AsyncDiskMmapFile;
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_disk_open_exec_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_exec_with_options_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    ///
    /// // mmap the file
    /// let mut opts = AsyncOptions::new();
    /// opts
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// // mmap the file
    /// let mut file = AsyncDiskMmapFile::open_exec_with_options("async_disk_open_exec_with_options_test.txt", opts).await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: structs.AsyncOptions.html
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
        if self.is_cow() {
            return Err(Error::TruncationFailed(String::from("cannot truncate a copy-on-write mmap file")));
        }

        // sync data
        let meta = self.file.metadata().await.map_err(Error::IO)?;
        if meta.len() > 0 {
            self.flush()?;
        }

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
        if self.is_cow() {
            return Err(Error::TruncationFailed(String::from("cannot truncate a copy-on-write mmap file")));
        }

        // sync data
        self.flush()?;

        // truncate
        self.file.set_len(max_sz).await.map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", self.path(), e)))?;

        // remap
        self.mmap = remmap(self.path(), &self.file, self.opts.as_ref(), self.typ)?;

        Ok(())
    }

    /// Remove the underlying file
    ///
    /// # Example
    ///
    /// ```rust
    /// use fmmap::AsyncMmapFileMutExt;
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncDiskMmapFileMut::create("async_disk_remove_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_remove_test.txt").unwrap());
    /// file.truncate(100).await;
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// # })
    /// ```
    async fn remove(mut self) -> crate::error::Result<()> {
        let path = self.path;
        drop(self.mmap);
        self.file.set_len(0).await.map_err(Error::IO)?;
        drop(self.file);
        remove_file(path).await.map_err(Error::IO)?;
        Ok(())
    }

    /// Close and truncate the underlying file
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{MetaDataExt, AsyncMmapFileExt, AsyncMmapFileMutExt};
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncDiskMmapFileMut::create("disk_close_with_truncate_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("disk_close_with_truncate_test.txt").unwrap());
    /// file.truncate(100).await;
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.close_with_truncate(50).await.unwrap();
    ///
    /// let file = AsyncDiskMmapFileMut::open("disk_close_with_truncate_test.txt").await.unwrap();
    /// let meta = file.metadata().await.unwrap();
    /// assert_eq!(meta.len(), 50);
    /// # })
    /// ```
    #[cfg(not(target_os = "linux"))]
    async fn close_with_truncate(self, max_sz: i64) -> crate::error::Result<()> {
        // sync data
        let meta = self.file.metadata().await.map_err(Error::IO)?;
        if meta.len() > 0 {
            self.flush()?;
        }

        drop(self.mmap);
        if max_sz >= 0 {
            self.file.set_len(max_sz as u64).await.map_err(Error::IO)?;
            let abs = self.path.canonicalize().map_err(Error::IO)?;
            let parent = abs.parent().unwrap();
            sync_dir_async(parent).await?;
        }
        Ok(())
    }

    /// Close and truncate the underlying file
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{MetaDataExt, AsyncMmapFileExt, AsyncMmapFileMutExt};
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncDiskMmapFileMut::create("disk_close_with_truncate_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("disk_close_with_truncate_test.txt").unwrap());
    /// file.truncate(100).await;
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.close_with_truncate(50).await.unwrap();
    ///
    /// let file = AsyncDiskMmapFileMut::open("disk_close_with_truncate_test.txt").await.unwrap();
    /// let meta = file.metadata().await.unwrap();
    /// assert_eq!(meta.len(), 50);
    /// # })
    /// ```
    #[cfg(target_os = "linux")]
    async fn close_with_truncate(self, max_sz: i64) -> crate::error::Result<()> {
        // sync data
        self.flush()?;
        drop(self.mmap);

        if max_sz >= 0 {
            self.file.set_len(max_sz as u64).await.map_err(Error::IO)?;
            let abs = self.path.canonicalize().map_err(Error::IO)?;
            let parent = abs.parent().unwrap();
            sync_dir_async(parent).await?;
        }
        Ok(())
    }
}

impl AsyncDiskMmapFileMut {
    /// Create a new file and mmap this file
    ///
    /// # Notes
    /// The new file is zero size, so, before write, you should truncate first.
    /// Or you can use [`create_with_options`] and set `max_size` field for [`AsyncOptions`] to enable directly write
    /// without truncating.
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::AsyncMmapFileMutExt;
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncDiskMmapFileMut::create("disk_create_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("disk_create_test.txt").unwrap());
    /// file.truncate(12).await;
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// # })
    /// ```
    ///
    /// [`create_with_options`]: structs.AsyncDiskMmapFileMut.html#method.create_with_options
    /// [`AsyncOptions`]: structs.AsyncOptions.html
    pub async fn create<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::create_in(path, None).await
    }

    /// Create a new file and mmap this file with [`Options`]
    ///
    /// ```rust
    /// use fmmap::{AsyncOptions, AsyncMmapFileMutExt};
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut opts = AsyncOptions::new();
    /// opts
    ///     // truncate to 100
    ///     .max_size(100);
    /// let mut file = AsyncDiskMmapFileMut::create_with_options("async_disk_create_with_options_test.txt", opts).await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_create_with_options_test.txt").unwrap());
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// # })
    /// ```
    ///
    /// [`Options`]: structs.Options.html
    pub async fn create_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
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
    /// ```rust
    /// use fmmap::{AsyncMmapFileExt, AsyncMmapFileMutExt};
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_disk_open_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    ///
    /// // mmap the file
    /// let mut file = AsyncDiskMmapFileMut::open("async_disk_open_test.txt").await.unwrap();
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
    /// let mut file = File::open("async_disk_open_test.txt").await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// File does not exists
    ///
    /// ```no_run
    /// use fmmap::{AsyncMmapFileExt, AsyncMmapFileMutExt};
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // mmap the file
    /// let mut file = AsyncDiskMmapFileMut::open("async_disk_open_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_test.txt").unwrap());
    /// file.truncate(100).await.unwrap();
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
    /// let mut file = File::open("async_disk_open_test.txt").await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`open_with_options`]: structs.AsyncDiskMmapFileMut.html#method.open_with_options
    /// [`AsyncOptions`]: structs.AsyncOptions.html
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_in(path, None).await
    }

    /// Open or Create(if not exists) a file and mmap this file with [`AsyncOptions`].
    ///
    /// # Examples
    ///
    /// File already exists
    ///
    /// ```rust
    /// use fmmap::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// use tokio::fs::File;
    /// use std::io::SeekFrom;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_disk_open_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_with_options_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    ///
    /// // mmap the file
    /// let mut opts = AsyncOptions::new();
    /// opts
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
    /// let mut file = AsyncDiskMmapFileMut::open_with_options("async_disk_open_with_options_test.txt", opts).await.unwrap();
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
    /// let mut file = File::open("async_disk_open_with_options_test.txt").await.unwrap();
    /// // skip the sanity text
    /// tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start("sanity text".as_bytes().len() as u64)).await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// File does not exists
    ///
    /// ```no_run
    /// use fmmap::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // mmap the file with options
    /// let mut opts = AsyncOptions::new();
    /// opts
    ///     // allow read
    ///     .read(true)
    ///     // allow write
    ///     .write(true)
    ///     // allow append
    ///     .append(true)
    ///     // truncate to 100
    ///     .max_size(100);
    ///
    /// let mut file = AsyncDiskMmapFileMut::open_with_options("async_disk_open_with_options_test.txt", opts).await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_with_options_test.txt").unwrap());
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
    /// let mut file = File::open("async_disk_open_with_options_test.txt").await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: structs.AsyncOptions.html
    pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
        Self::open_in(path, Some(opts)).await
    }

    /// Open an existing file and mmap this file
    ///
    /// # Examples
    /// ```rust
    /// use fmmap::{AsyncMmapFileExt, AsyncMmapFileMutExt};
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // create a temp file
    /// let mut file = File::create("async_disk_open_existing_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_existing_test.txt").unwrap());
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = AsyncDiskMmapFileMut::open_exist("async_disk_open_existing_test.txt").await.unwrap();
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
    /// let mut file = File::open("async_disk_open_existing_test.txt").await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: structs.AsyncOptions.html
    pub async fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_exist_in(path, None).await
    }

    /// Open an existing file and mmap this file with [`AsyncOptions`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// use tokio::fs::File;
    /// use std::io::SeekFrom;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // create a temp file
    /// let mut file = File::create("async_disk_open_existing_test_with_options.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_existing_test_with_options.txt").unwrap());
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut opts = AsyncOptions::new();
    /// opts
    ///     // truncate to 100
    ///     .max_size(100)
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    ///
    /// let mut file = AsyncDiskMmapFileMut::open_exist_with_options("async_disk_open_existing_test_with_options.txt", opts).await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).await.unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    ///
    /// // reopen to check content, cow will not change the content.
    /// let mut file = File::open("async_disk_open_existing_test_with_options.txt").await.unwrap();
    /// let mut buf = vec![0; "some modified data...".len()];
    /// // skip the sanity text
    /// tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start("sanity text".as_bytes().len() as u64)).await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: structs.AsyncOptions.html
    pub async fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
        Self::open_exist_in(path, Some(opts)).await
    }

    /// Open and mmap an existing file in copy-on-write mode.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{AsyncMmapFileExt, AsyncMmapFileMutExt};
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // create a temp file
    /// let mut file = File::create("async_disk_open_cow_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_cow_test.txt").unwrap());
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = AsyncDiskMmapFileMut::open_cow("async_disk_open_cow_test.txt").await.unwrap();
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
    /// let mut file = File::open("async_disk_open_cow_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: structs.AsyncOptions.html
    pub async fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_cow_in(path, None).await
    }

    /// Open and mmap an existing file in copy-on-write mode with [`Options`].
    /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// use tokio::fs::File;
    /// use std::io::SeekFrom;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // create a temp file
    /// let mut file = File::create("async_disk_open_cow_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_open_cow_test.txt").unwrap());
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut opts = AsyncOptions::new();
    /// opts
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    ///
    /// let mut file = AsyncDiskMmapFileMut::open_cow_with_options("async_disk_open_cow_test.txt", opts).await.unwrap();
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
    /// let mut file = File::open("async_disk_open_cow_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// // skip the sanity text
    /// tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start("sanity text".as_bytes().len() as u64)).await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: structs.AsyncOptions.html
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::AsyncMmapFileMutExt;
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncDiskMmapFileMut::create("disk_create_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("disk_create_test.txt").unwrap());
    /// file.truncate(12).await;
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// // freeze
    /// file.freeze().unwrap();
    /// # })
    /// ```
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
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::AsyncMmapFileMutExt;
    /// use fmmap::raw::AsyncDiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncDiskMmapFileMut::create("disk_create_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("disk_create_test.txt").unwrap());
    /// file.truncate(12).await;
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// // freeze_exec
    /// file.freeze_exec().unwrap();
    /// # })
    /// ```
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
                    let abs = path.as_ref().canonicalize().map_err(Error::IO)?;
                    let parent = abs.parent().unwrap();
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
                let file = open_or_create_file_async(&path)
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
            Some(mut opts) => {
                let file = opts.file_opts.create(true).open(&path)
                    .await
                    .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;
                let meta = file.metadata().await?;
                let file_sz = meta.len();
                if file_sz == 0 && opts.max_size > 0 {
                    file.set_len(opts.max_size).await.map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", path.as_ref(), e)))?;
                    let abs = path.as_ref().canonicalize().map_err(Error::IO)?;
                    let parent = abs.parent().unwrap();
                    sync_dir_async(parent).await?;
                }

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
                    let abs = path.as_ref().canonicalize().map_err(Error::IO)?;
                    let parent = abs.parent().unwrap();
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

#[cfg(test)]
mod test {
    use super::*;
    use scopeguard::defer;

    #[tokio::test]
    async fn test_close_with_truncate_on_empty_file() {
        let file = AsyncDiskMmapFileMut::create("async_disk_close_with_truncate_test.txt").await.unwrap();
        defer!(std::fs::remove_file("async_disk_close_with_truncate_test.txt").unwrap());
        file.close_with_truncate(10).await.unwrap();

        assert_eq!(10, File::open("async_disk_close_with_truncate_test.txt").await.unwrap().metadata().await.unwrap().len());
    }
}