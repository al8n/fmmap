#[derive(Copy, Clone)]
enum MmapFileMutType {
    Cow,
    Normal,
}

macro_rules! remmap {
    ($path: ty) => {
        #[inline]
        fn remmap<T: MmapAsRawDesc>(
            path: &$path,
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
                .map_err(|e| {
                    Error::new_source_msg(ErrorKind::RemmapFailed, path.to_string_lossy(), e)
                })
            }
        }
    };
}

macro_rules! impl_flush {
    () => {
        fn flush(&self) -> crate::error::Result<()> {
            self.mmap
                .flush()
                .map_err(|e| Error::new_source_msg(ErrorKind::FlushFailed, self.path_string(), e))
        }

        fn flush_async(&self) -> crate::error::Result<()> {
            self.mmap
                .flush_async()
                .map_err(|e| Error::new_source_msg(ErrorKind::FlushFailed, self.path_string(), e))
        }

        fn flush_range(&self, offset: usize, len: usize) -> crate::error::Result<()> {
            self.mmap
                .flush_range(offset, len)
                .map_err(|e| Error::new_source_msg(ErrorKind::FlushFailed, self.path_string(), e))
        }

        fn flush_async_range(&self, offset: usize, len: usize) -> crate::error::Result<()> {
            self.mmap
                .flush_async_range(offset, len)
                .map_err(|e| Error::new_source_msg(ErrorKind::FlushFailed, self.path_string(), e))
        }
    };
}

macro_rules! impl_file_lock {
    () => {
        #[inline]
        fn lock_exclusive(&self) -> crate::error::Result<()> {
            self.file
                .lock_exclusive()
                .map_err(|e| Error::new(ErrorKind::IO, e))
        }

        #[inline]
        #[allow(warnings)]
        fn lock_shared(&self) -> crate::error::Result<()> {
            self.file
                .lock_shared()
                .map_err(|e| Error::new(ErrorKind::IO, e))
        }

        #[inline]
        fn try_lock_exclusive(&self) -> crate::error::Result<()> {
            self.file
                .try_lock_exclusive()
                .map(|_| ())
                .map_err(|e| Error::new(ErrorKind::IO, e))
        }

        #[inline]
        #[allow(warnings)]
        fn try_lock_shared(&self) -> crate::error::Result<()> {
            self.file
                .try_lock_shared()
                .map(|_| ())
                .map_err(|e| Error::new(ErrorKind::IO, e))
        }

        #[inline]
        #[allow(warnings)]
        fn unlock(&self) -> crate::error::Result<()> {
            self.file.unlock().map_err(|e| Error::new(ErrorKind::IO, e))
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
                self.file.metadata().map(MetaData::disk).map_err(|e| Error::new(ErrorKind::IO, e))
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
                        .map_err(|e| Error::new(ErrorKind::IO, e))
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
                        .map_err(|e| Error::new(ErrorKind::IO, e))
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
                pub async fn open<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
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
                pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
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
                pub async fn open_exec<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
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
                pub async fn open_exec_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
                    Self::open_exec_in(path, Some(opts)).await
                }

                async fn open_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
                    let file = open_read_only_file_async(&path).await.map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e))?;

                    match opts  {
                        None => {
                            let mmap = unsafe {
                                Mmap::map(&file).map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
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
                                opts.mmap_opts.map(&file).map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
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
                        .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e))?;

                    match opts  {
                        None => {
                            let mmap = unsafe {
                                MmapOptions::new().map_exec(&file).map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
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
                                opts.mmap_opts.map_exec(&file).map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
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
                        return Err(Error::new_with_message(ErrorKind::TruncationFailed, "cannot truncate a copy-on-write mmap file"));
                    }

                    // sync data
                    let meta = self.file.metadata().await.map_err(|e| Error::new(ErrorKind::IO, e))?;
                    if meta.len() > 0 {
                        self.flush()?;
                    }

                    unsafe {
                        // unmap
                        drop_in_place(&mut self.mmap);

                        // truncate
                        self.file.set_len(max_sz).await.map_err(|e| Error::new_source_msg(ErrorKind::TruncationFailed, self.path_lossy(), e))?;

                        // remap
                        let mmap = remmap(self.path(), &self.file, self.opts.as_ref(), self.typ)?;

                        write(&mut self.mmap, mmap);
                    }

                    Ok(())
                }

                #[cfg(target_os = "linux")]
                async fn truncate(&mut self, max_sz: u64) -> Result<(), Error> {
                    if self.is_cow() {
                        return Err(Error::new_with_message(ErrorKind::TruncationFailed, "cannot truncate a copy-on-write mmap file"));
                    }

                    // sync data
                    self.flush()?;

                    // truncate
                    self.file.set_len(max_sz).await.map_err(|e| Error::new_source_msg(ErrorKind::TruncationFailed, self.path_lossy(), e))?;

                    // remap
                    self.mmap = remmap(self.path(), &self.file, self.opts.as_ref(), self.typ)?;

                    Ok(())
                }

                /// Remove the underlying file
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
                    drop(self.mmap);
                    self.file.set_len(0).await.map_err(|e| Error::new(ErrorKind::IO, e))?;
                    drop(self.file);
                    remove_file(path).await.map_err(|e| Error::new(ErrorKind::IO, e))?;
                    Ok(())
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
                    #[cfg(not(target_os = "linux"))]
                    {
                        // sync data
                        let meta = self.file.metadata().await.map_err(|e| Error::new(ErrorKind::IO, e))?;
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
                        self.file.set_len(max_sz as u64).await.map_err(|e| Error::new(ErrorKind::IO, e))?;
                        sync_parent_async(&self.path).await?;
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
                typ: MmapFileMutType,
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
                pub async fn create<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
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
                pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
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
                pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
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
                pub async fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
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
                pub async fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self, Error> {
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
                pub async fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
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
                    Ok($immutable_file {
                        mmap: self.mmap.make_read_only().map_err(|e| Error::new(ErrorKind::IO, e))?,
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
                    Ok($immutable_file {
                        mmap: self.mmap.make_exec().map_err(|e| Error::new(ErrorKind::IO, e))?,
                        file: self.file,
                        path: self.path,
                        exec: true
                    })
                }
            }
        };
    }

    macro_rules! impl_async_fmmap_file_mut_private {
        ($name: ident) => {
            impl $name {
                async fn create_in<P: AsRef<Path>>(path: P, opts: Option<AsyncOptions>) -> Result<Self, Error> {
                    let file = create_file_async(&path)
                        .await
                        .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e))?;

                    match opts {
                        None => {
                            let mmap = unsafe { MmapMut::map_mut(&file).map_err(|e| Error::new(ErrorKind::MmapFailed, e))? };

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
                                file.set_len(opts.max_size).await.map_err(|e| Error::new_source_msg(ErrorKind::TruncationFailed, path.as_ref().to_string_lossy(), e))?;
                                sync_parent_async(&path).await?;
                            }

                            let opts_bk = opts.mmap_opts.clone();
                            let mmap = unsafe { opts.mmap_opts.map_mut(&file).map_err(|e| Error::new(ErrorKind::MmapFailed, e))? };

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
                                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e))?;

                            let mmap = unsafe { MmapMut::map_mut(&file).map_err(|e| Error::new(ErrorKind::MmapFailed, e))? };
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
                                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e))?;
                            let meta = file.metadata().await?;
                            let file_sz = meta.len();
                            if file_sz == 0 && opts.max_size > 0 {
                                file.set_len(opts.max_size).await.map_err(|e| Error::new_source_msg(ErrorKind::TruncationFailed, path.as_ref().to_string_lossy(), e))?;
                                sync_parent_async(&path).await?;
                            }

                            let opts_bk = opts.mmap_opts.clone();
                            let mmap = unsafe {
                                opts.mmap_opts.map_mut(&file).map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
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
                        .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e))?;

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
                                file.set_len(opts.max_size).await.map_err(|e| Error::new_source_msg(ErrorKind::TruncationFailed, path.as_ref().to_string_lossy(), e))?;
                                sync_parent_async(&path).await?;
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
                        .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e))?;

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
