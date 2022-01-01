use crate::error::Error;
use memmap2::{MmapAsRawDesc, MmapMut, MmapOptions};
use std::path::Path;

#[derive(Copy, Clone)]
enum MmapFileMutType {
    Cow,
    Normal,
}

#[inline]
fn remmap<T: MmapAsRawDesc>(
    path: &Path,
    file: T,
    opts: Option<&MmapOptions>,
    typ: MmapFileMutType,
) -> Result<MmapMut, Error> {
    unsafe {
        match opts {
            None => match typ {
                MmapFileMutType::Cow => MmapOptions::new().map_copy(file),
                MmapFileMutType::Normal => MmapMut::map_mut(file),
            },
            Some(opts) => {
                let opts = opts.clone();
                match typ {
                    MmapFileMutType::Cow => opts.map_copy(file),
                    MmapFileMutType::Normal => opts.map_mut(file),
                }
            }
        }
        .map_err(|e| Error::RemmapFailed(format!("path: {:?}, err: {}", path, e)))
    }
}

macro_rules! impl_flush {
    () => {
        fn flush(&self) -> crate::error::Result<()> {
            self.mmap
                .flush()
                .map_err(|e| Error::FlushFailed(format!("path: {:?}, err: {}", self.path(), e)))
        }

        fn flush_async(&self) -> crate::error::Result<()> {
            self.mmap
                .flush_async()
                .map_err(|e| Error::FlushFailed(format!("path: {:?}, err: {}", self.path(), e)))
        }

        fn flush_range(&self, offset: usize, len: usize) -> crate::error::Result<()> {
            self.mmap
                .flush_range(offset, len)
                .map_err(|e| Error::FlushFailed(format!("path: {:?}, err: {}", self.path(), e)))
        }

        fn flush_async_range(&self, offset: usize, len: usize) -> crate::error::Result<()> {
            self.mmap
                .flush_async_range(offset, len)
                .map_err(|e| Error::FlushFailed(format!("path: {:?}, err: {}", self.path(), e)))
        }
    };
}

macro_rules! impl_file_lock {
    () => {
        #[inline]
        fn lock_exclusive(&self) -> crate::error::Result<()> {
            self.file.lock_exclusive().map_err(Error::IO)
        }

        #[inline]
        fn lock_shared(&self) -> crate::error::Result<()> {
            self.file.lock_shared().map_err(Error::IO)
        }

        #[inline]
        fn try_lock_exclusive(&self) -> crate::error::Result<()> {
            self.file.try_lock_exclusive().map_err(Error::IO)
        }

        #[inline]
        fn try_lock_shared(&self) -> crate::error::Result<()> {
            self.file.try_lock_shared().map_err(Error::IO)
        }

        #[inline]
        fn unlock(&self) -> crate::error::Result<()> {
            self.file.unlock().map_err(Error::IO)
        }
    };
}

cfg_sync! {
    macro_rules! impl_mmap_file_ext_base {
        () => {
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
                self.file.metadata().map(MetaData::disk).map_err(Error::IO)
            }

            impl_file_lock!();
        };
    }

    macro_rules! impl_mmap_file_ext {
        ($name: ident) => {
            impl MmapFileExt for $name {
                impl_mmap_file_ext_base!();

                /// Whether the mmap is executable.
                #[inline]
                fn is_exec(&self) -> bool {
                    self.exec
                }
            }
        };
    }

    macro_rules! impl_mmap_file_ext_for_mut {
        ($name: ident) => {
            impl MmapFileExt for $name {
                impl_mmap_file_ext_base!();

                /// Whether the mmap is executable.
                #[inline]
                fn is_exec(&self) -> bool {
                    false
                }
            }
        };
    }

    mod sync_impl;
    pub use sync_impl::{DiskMmapFile, DiskMmapFileMut};
}

macro_rules! declare_and_impl_async_fmmap_file_mut {
    ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal, $base_file: ty) => {
        /// AsyncDiskMmapFileMut contains a mutable mmap buffer
        /// and a writable file.
        pub struct AsyncDiskMmapFileMut {
            pub(crate) mmap: MmapMut,
            pub(crate) file: $base_file,
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
            /// use fmmap::tokio::AsyncMmapFileMutExt;
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            ///
            /// # tokio_test::block_on(async {
            /// let mut file = AsyncDiskMmapFileMut::create("tokio_async_disk_remove_test.txt").await.unwrap();
            ///
            /// file.truncate(12).await;
            /// file.write_all("some data...".as_bytes(), 0).unwrap();
            /// file.flush().unwrap();
            ///
            /// file.remove().await.unwrap();
            ///
            /// let err = tokio::fs::File::open("tokio_async_disk_remove_test.txt").await;
            /// assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
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
            /// use fmmap::MetaDataExt;
            /// use fmmap::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt};
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
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
            /// use fmmap::{MetaDataExt,  tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt}};
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
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
            /// use fmmap::tokio::AsyncMmapFileMutExt;
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
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
            /// [`create_with_options`]: struct.AsyncDiskMmapFileMut.html#method.create_with_options
            /// [`AsyncOptions`]: struct.AsyncOptions.html
            pub async fn create<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
                Self::create_in(path, None).await
            }

            /// Create a new file and mmap this file with [`AsyncOptions`]
            ///
            /// ```rust
            /// use fmmap::tokio::{AsyncOptions, AsyncMmapFileMutExt};
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            /// # use scopeguard::defer;
            ///
            /// # tokio_test::block_on(async {
            /// let opts = AsyncOptions::new()
            ///     // truncate to 100
            ///     .max_size(100);
            /// let mut file = AsyncDiskMmapFileMut::create_with_options("tokio_async_disk_create_with_options_test.txt", opts).await.unwrap();
            /// # defer!(std::fs::remove_file("tokio_async_disk_create_with_options_test.txt").unwrap());
            /// file.write_all("some data...".as_bytes(), 0).unwrap();
            /// file.flush().unwrap();
            /// # })
            /// ```
            ///
            /// [`AsyncOptions`]: struct.AsyncOptions.html
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
            /// use fmmap::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt};
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            /// use tokio::fs::File;
            /// # use scopeguard::defer;
            ///
            /// # tokio_test::block_on(async {
            /// # let mut file = File::create("tokio_async_disk_open_test.txt").await.unwrap();
            /// # defer!(std::fs::remove_file("tokio_async_disk_open_test.txt").unwrap());
            /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
            /// # drop(file);
            ///
            /// // mmap the file
            /// let mut file = AsyncDiskMmapFileMut::open("tokio_async_disk_open_test.txt").await.unwrap();
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
            /// let mut file = File::open("tokio_async_disk_open_test.txt").await.unwrap();
            /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
            /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
            /// # })
            /// ```
            ///
            /// File does not exists
            ///
            /// ```no_run
            /// use fmmap::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt};
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            /// use tokio::fs::File;
            /// # use scopeguard::defer;
            ///
            /// # tokio_test::block_on(async {
            /// // mmap the file
            /// let mut file = AsyncDiskMmapFileMut::open("tokio_async_disk_open_test.txt").await.unwrap();
            /// # defer!(std::fs::remove_file("tokio_async_disk_open_test.txt").unwrap());
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
            /// let mut file = File::open("tokio_async_disk_open_test.txt").await.unwrap();
            /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
            /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
            /// # })
            /// ```
            ///
            /// [`open_with_options`]: struct.AsyncDiskMmapFileMut.html#method.open_with_options
            /// [`AsyncOptions`]: struct.AsyncOptions.html
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
            /// use fmmap::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            /// use tokio::fs::File;
            /// use std::io::SeekFrom;
            /// # use scopeguard::defer;
            ///
            /// # tokio_test::block_on(async {
            /// # let mut file = File::create("tokio_async_disk_open_with_options_test.txt").await.unwrap();
            /// # defer!(std::fs::remove_file("tokio_async_disk_open_with_options_test.txt").unwrap());
            /// # tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
            /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
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
            /// let mut file = AsyncDiskMmapFileMut::open_with_options("tokio_async_disk_open_with_options_test.txt", opts).await.unwrap();
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
            /// let mut file = File::open("tokio_async_disk_open_with_options_test.txt").await.unwrap();
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
            /// use fmmap::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            /// use tokio::fs::File;
            /// # use scopeguard::defer;
            ///
            /// # tokio_test::block_on(async {
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
            /// let mut file = AsyncDiskMmapFileMut::open_with_options("tokio_async_disk_open_with_options_test.txt", opts).await.unwrap();
            /// # defer!(std::fs::remove_file("tokio_async_disk_open_with_options_test.txt").unwrap());
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
            /// let mut file = File::open("tokio_async_disk_open_with_options_test.txt").await.unwrap();
            /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
            /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
            /// # })
            /// ```
            ///
            /// [`AsyncOptions`]: struct.AsyncOptions.html
            pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
                Self::open_in(path, Some(opts)).await
            }

            /// Open an existing file and mmap this file
            ///
            /// # Examples
            /// ```rust
            /// use fmmap::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt};
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            /// use tokio::fs::File;
            /// # use scopeguard::defer;
            ///
            /// # tokio_test::block_on(async {
            /// // create a temp file
            /// let mut file = File::create("tokio_async_disk_open_existing_test.txt").await.unwrap();
            /// # defer!(std::fs::remove_file("tokio_async_disk_open_existing_test.txt").unwrap());
            /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
            /// drop(file);
            ///
            /// // mmap the file
            /// let mut file = AsyncDiskMmapFileMut::open_exist("tokio_async_disk_open_existing_test.txt").await.unwrap();
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
            /// let mut file = File::open("tokio_async_disk_open_existing_test.txt").await.unwrap();
            /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
            /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
            /// # })
            /// ```
            ///
            /// [`AsyncOptions`]: struct.AsyncOptions.html
            pub async fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
                Self::open_exist_in(path, None).await
            }

            /// Open an existing file and mmap this file with [`AsyncOptions`]
            ///
            /// # Examples
            ///
            /// ```rust
            /// use fmmap::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            /// use tokio::fs::File;
            /// use std::io::SeekFrom;
            /// # use scopeguard::defer;
            ///
            /// # tokio_test::block_on(async {
            /// // create a temp file
            /// let mut file = File::create("tokio_async_disk_open_existing_test_with_options.txt").await.unwrap();
            /// # defer!(std::fs::remove_file("tokio_async_disk_open_existing_test_with_options.txt").unwrap());
            /// tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
            /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
            /// drop(file);
            ///
            /// // mmap the file
            /// let opts = AsyncOptions::new()
            ///     // truncate to 100
            ///     .max_size(100)
            ///     // mmap content after the sanity text
            ///     .offset("sanity text".as_bytes().len() as u64);
            ///
            /// let mut file = AsyncDiskMmapFileMut::open_exist_with_options("tokio_async_disk_open_existing_test_with_options.txt", opts).await.unwrap();
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
            /// let mut file = File::open("tokio_async_disk_open_existing_test_with_options.txt").await.unwrap();
            /// let mut buf = vec![0; "some modified data...".len()];
            /// // skip the sanity text
            /// tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start("sanity text".as_bytes().len() as u64)).await.unwrap();
            /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
            /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
            /// # })
            /// ```
            ///
            /// [`AsyncOptions`]: struct.AsyncOptions.html
            pub async fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
                Self::open_exist_in(path, Some(opts)).await
            }

            /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file).
            /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
            ///
            /// # Examples
            ///
            /// ```rust
            /// use fmmap::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt};
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            /// use tokio::fs::File;
            /// # use scopeguard::defer;
            ///
            /// # tokio_test::block_on(async {
            /// // create a temp file
            /// let mut file = File::create("tokio_async_disk_open_cow_test.txt").await.unwrap();
            /// # defer!(std::fs::remove_file("tokio_async_disk_open_cow_test.txt").unwrap());
            /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
            /// drop(file);
            ///
            /// // mmap the file
            /// let mut file = AsyncDiskMmapFileMut::open_cow("tokio_async_disk_open_cow_test.txt").await.unwrap();
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
            /// let mut file = File::open("tokio_async_disk_open_cow_test.txt").await.unwrap();
            /// let mut buf = vec![0; "some data...".len()];
            /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
            /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
            /// # })
            /// ```
            ///
            /// [`AsyncOptions`]: struct.AsyncOptions.html
            pub async fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
                Self::open_cow_in(path, None).await
            }

            /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file) with [`AsyncOptions`].
            /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
            ///
            /// # Examples
            ///
            /// ```rust
            /// use fmmap::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            /// use tokio::fs::File;
            /// use std::io::SeekFrom;
            /// # use scopeguard::defer;
            ///
            /// # tokio_test::block_on(async {
            /// // create a temp file
            /// let mut file = File::create("tokio_async_disk_open_cow_with_options_test.txt").await.unwrap();
            /// # defer!(std::fs::remove_file("tokio_async_disk_open_cow_with_options_test.txt").unwrap());
            /// tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
            /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
            /// drop(file);
            ///
            /// // mmap the file
            /// let opts = AsyncOptions::new()
            ///     // mmap content after the sanity text
            ///     .offset("sanity text".as_bytes().len() as u64);
            ///
            /// let mut file = AsyncDiskMmapFileMut::open_cow_with_options("tokio_async_disk_open_cow_with_options_test.txt", opts).await.unwrap();
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
            /// let mut file = File::open("tokio_async_disk_open_cow_with_options_test.txt").await.unwrap();
            /// let mut buf = vec![0; "some data...".len()];
            /// // skip the sanity text
            /// tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start("sanity text".as_bytes().len() as u64)).await.unwrap();
            /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
            /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
            /// # })
            /// ```
            ///
            /// [`AsyncOptions`]: struct.AsyncOptions.html
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
            /// use fmmap::tokio::AsyncMmapFileMutExt;
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            /// # use scopeguard::defer;
            ///
            /// # tokio_test::block_on(async {
            /// let mut file = AsyncDiskMmapFileMut::create("tokio_async_disk_freeze_test.txt").await.unwrap();
            /// # defer!(std::fs::remove_file("tokio_async_disk_freeze_test.txt").unwrap());
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
            /// use fmmap::tokio::AsyncMmapFileMutExt;
            /// use fmmap::raw::tokio::AsyncDiskMmapFileMut;
            /// # use scopeguard::defer;
            ///
            /// # tokio_test::block_on(async {
            /// let mut file = AsyncDiskMmapFileMut::create("tokio_async_disk_freeze_exec_test.txt").await.unwrap();
            /// # defer!(std::fs::remove_file("tokio_async_disk_freeze_exec_test.txt").unwrap());
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
    };
}


cfg_async! {
    macro_rules! impl_async_mmap_file_ext {
        ($name: ident) => {
            #[async_trait]
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
                        .map_err(Error::IO)
                }

                /// Whether the mmap is executable.
                #[inline]
                fn is_exec(&self) -> bool {
                    self.exec
                }

                impl_file_lock!();
            }
        };
    }

    macro_rules! impl_async_mmap_file_ext_for_mut {
        ($name: ident) => {
            #[async_trait]
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
                        .map_err(Error::IO)
                }

                /// Whether the mmap is executable.
                #[inline]
                fn is_exec(&self) -> bool {
                    false
                }

                impl_file_lock!();
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
            }

            impl_async_mmap_file_ext!(AsyncDiskMmapFile);

            impl AsyncDiskMmapFile {
                /// Open a readable memory map backed by a file
                ///
                /// # Examples
                ///
                /// ```rust
                #[doc = concat!("use fmmap::", $path_str, "AsyncMmapFileExt;")]
                #[doc = concat!("use fmmap::raw::", $path_str, "AsyncDiskMmapFile;")]
                #[doc = concat!("use ", $path_str, "fs::File;")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = File::create(\"", $filename_prefix, "_disk_open_test.txt\").await.unwrap();")]
                #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_test.txt\").unwrap());")]
                #[doc = concat!("# ", $path_str, "io::AsyncWriteExt::write_all(&mut file, \"some data...\".as_bytes()).await.unwrap();")]
                /// # drop(file);
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncDiskMmapFile::open(\"", $filename_prefix, "_disk_open_test.txt\").await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                /// ```
                pub async fn open<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
                    Self::open_in(path, None).await
                }

                /// Open a readable memory map backed by a file with [`AsyncOptions`]
                ///
                /// # Examples
                ///
                /// ```rust
                #[doc = concat!("use fmmap::", $path_str, "{AsyncOptions, AsyncMmapFileExt};")]
                #[doc = concat!("use fmmap::raw::", $path_str, "AsyncDiskMmapFile;")]
                #[doc = concat!("use ", $path_str, "fs::File;")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = File::create(\"", $filename_prefix, "_disk_open_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_with_options_test.txt\").unwrap());")]
                #[doc = concat!("# ", $path_str, "io::AsyncWriteExt::write_all(&mut file, \"sanity text\".as_bytes()).await.unwrap();")]
                #[doc = concat!("# ", $path_str, "io::AsyncWriteExt::write_all(&mut file, \"some data...\".as_bytes()).await.unwrap();")]
                /// # drop(file);
                ///
                /// // mmap the file
                /// let opts = AsyncOptions::new()
                ///     // mmap content after the sanity text
                ///     .offset("sanity text".as_bytes().len() as u64);
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncDiskMmapFile::open_with_options(\"", $filename_prefix, "_disk_open_with_options_test.txt\", opts).await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                /// [AsyncOptions`]: tokio/struct.AsyncOptions.html
                pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
                    Self::open_in(path, Some(opts)).await
                }

                /// Open a readable and executable memory map backed by a file
                ///
                /// # Examples
                ///
                /// ```rust
                #[doc = concat!("use fmmap::", $path_str, "AsyncMmapFileExt;")]
                #[doc = concat!("use fmmap::raw::", $path_str, "AsyncDiskMmapFile;")]
                #[doc = concat!("use ", $path_str, "fs::File;")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = File::create(\"", $filename_prefix, "_disk_open_exec_test.txt\").await.unwrap();")]
                #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_exec_test.txt\").unwrap());")]
                #[doc = concat!("# ", $path_str, "io::AsyncWriteExt::write_all(&mut file, \"some data...\".as_bytes()).await.unwrap();")]
                /// # drop(file);
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncDiskMmapFile::open_exec(\"", $filename_prefix, "_disk_open_exec_test.txt\").await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                /// ```
                pub async fn open_exec<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
                    Self::open_exec_in(path, None).await
                }

                /// Open a readable and executable memory map backed by a file with [`AsyncOptions`].
                ///
                /// # Examples
                ///
                /// ```rust
                #[doc = concat!("use fmmap::", $path_str, "{AsyncOptions, AsyncMmapFileExt};")]
                #[doc = concat!("use fmmap::raw::", $path_str, "AsyncDiskMmapFile;")]
                #[doc = concat!("use ", $path_str, "fs::File;")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = File::create(\"", $filename_prefix, "_disk_open_exec_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_disk_open_exec_with_options_test.txt\").unwrap());")]
                #[doc = concat!("# ", $path_str, "io::AsyncWriteExt::write_all(&mut file, \"sanity text\".as_bytes()).await.unwrap();")]
                #[doc = concat!("# ", $path_str, "io::AsyncWriteExt::write_all(&mut file, \"some data...\".as_bytes()).await.unwrap();")]
                /// # drop(file);
                ///
                /// // mmap the file
                /// let opts = AsyncOptions::new()
                ///     // mmap content after the sanity text
                ///     .offset("sanity text".as_bytes().len() as u64);
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncDiskMmapFile::open_exec_with_options(\"", $filename_prefix, "_disk_open_exec_with_options_test.txt\", opts).await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                /// [`AsyncOptions`]: tokio/struct.AsyncOptions.html
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
        };
    }

    macro_rules! impl_async_mmap_file_mut_ext_for_mut {
        ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal) => {
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
                #[doc = concat!("use fmmap::", $path_str, "AsyncMmapFileMutExt;")]
                #[doc = concat!("use fmmap::raw::", $path_str, "AsyncDiskMmapFileMut;")]
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_disk_remove_test.txt\").await.unwrap();")]
                ///
                /// file.truncate(12).await;
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
                ///
                /// file.remove().await.unwrap();
                ///
                #[doc = concat!("let err = ", $path_str, "fs::File::open(\"tokio_async_disk_remove_test.txt\").await;")]
                /// assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
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
                /// use fmmap::MetaDataExt;
                #[doc = concat!("use fmmap::", $path_str, "{AsyncMmapFileExt, AsyncMmapFileMutExt};")]
                #[doc = concat!("use fmmap::raw::", $path_str, "AsyncDiskMmapFileMut;")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("let mut file = AsyncDiskMmapFileMut::create(\"", $filename_prefix, "_close_with_truncate_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_close_with_truncate_test.txt\").unwrap());")]
                /// file.truncate(100).await;
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
                ///
                /// file.close_with_truncate(50).await.unwrap();
                ///
                #[doc = concat!("let file = AsyncDiskMmapFileMut::open(\"", $filename_prefix, "_close_with_truncate_test.txt\").await.unwrap();")]
                /// let meta = file.metadata().await.unwrap();
                /// assert_eq!(meta.len(), 50);
                /// # })
                /// ```
                async fn close_with_truncate(self, max_sz: i64) -> crate::error::Result<()> {
                    #[cfg(not(target_os = "linux"))]
                    {
                        // sync data
                        let meta = self.file.metadata().await.map_err(Error::IO)?;
                        if meta.len() > 0 {
                            self.flush()?;
                        }
                    }
                    #[cfg(target_os = "linux")]
                    {
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
            }
        };
    }
}

cfg_tokio! {
    pub(crate) mod tokio_impl;
}

cfg_smol! {
    pub(crate) mod smol_impl;
}

cfg_async_std! {
    pub(crate) mod async_std_impl;
}