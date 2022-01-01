macro_rules! declare_and_impl_options {
    ($name: ident, $file_open_options: ident) => {
        /// A memory map builder, providing advanced options and flags for specifying memory map file behavior.
        ///
        // TODO: support file lock options
        #[derive(Clone)]
        pub struct $name {
            pub(crate) mmap_opts: MmapOptions,
            pub(crate) file_opts: $file_open_options,
            pub(crate) max_size: u64,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $name {
            /// Creates a new set of options for configuring and creating a memory map.
            pub fn new() -> Self {
                Self {
                    mmap_opts: MmapOptions::new(),
                    file_opts: <$file_open_options>::new(),
                    max_size: 0,
                }
            }

            /// Configures the memory map to start at byte offset from the beginning of the file.
            /// This option has no effect on anonymous memory maps.
            /// By default, the offset is 0.
            pub fn offset(mut self, offset: u64) -> Self {
                self.mmap_opts.offset(offset);
                self
            }

            /// Configures the created memory mapped buffer to be len bytes long.
            /// This option is mandatory for anonymous memory maps.
            /// For file-backed memory maps, the length will default to the file length.
            pub fn len(mut self, len: usize) -> Self {
                self.mmap_opts.len(len);
                self
            }

            /// Populate (prefault) page tables for a mapping.
            /// For a file mapping, this causes read-ahead on the file. This will help to reduce blocking on page faults later.
            /// This option corresponds to the MAP_POPULATE flag on Linux. It has no effect on Windows
            pub fn populate(mut self) -> Self {
                self.mmap_opts.populate();
                self
            }

            /// Configures the anonymous memory map to be suitable for a process or thread stack.
            /// This option corresponds to the MAP_STACK flag on Linux. It has no effect on Windows.
            /// This option has no effect on file-backed memory maps
            pub fn stack(mut self) -> Self {
                self.mmap_opts.stack();
                self
            }

            /// Configures the max size of the file.
            ///
            /// This option only has effect when mmaping a real file in write mode.
            ///
            /// This field is ignored when opening [`DiskMmapFile`], [`AsyncDiskMmapFile`], [`MmapFile`] and [`AsyncMmapFile`].
            ///
            /// [`DiskMmapFile`]: fmmap::raw::DiskMmapFile
            /// [`AsyncDiskMmapFile`]: fmmap::raw::AsyncDiskMmapFile
            /// [`MmapFile`]: struct.MmapFile.html
            /// [`AsyncMmapFile`]: struct.AsyncMmapFile.html
            pub fn max_size(mut self, max_sz: u64) -> Self {
                self.max_size = max_sz;
                self
            }

            /// Sets the option for read access. For details, please see [`std::fs::OpenOptions::read`]
            ///
            /// [`std::fs::OpenOptions::read`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.read
            pub fn read(mut self, val: bool) -> Self {
                self.file_opts.read(val);
                self
            }

            /// Sets the option for write access. For details, please see [`std::fs::OpenOptions::write`].
            ///
            /// This field is ignored when opening [`DiskMmapFile`], [`AsyncDiskMmapFile`], [`MmapFile`] and [`AsyncMmapFile`].
            ///
            /// [`DiskMmapFile`]: fmmap::raw::DiskMmapFile
            /// [`AsyncDiskMmapFile`]: fmmap::raw::AsyncDiskMmapFile
            /// [`MmapFile`]: struct.MmapFile.html
            /// [`AsyncMmapFile`]: struct.AsyncMmapFile.html
            /// [`std::fs::OpenOptions::write`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.write
            pub fn write(mut self, val: bool) -> Self {
                self.file_opts.write(val);
                self
            }

            /// Sets the option to create a new file, or open it if it already exists. For details, please see [`std::fs::OpenOptions::create`].
            ///
            /// This field is ignored when opening [`DiskMmapFile`], [`AsyncDiskMmapFile`], [`MmapFile`] and [`AsyncMmapFile`].
            ///
            /// [`DiskMmapFile`]: fmmap::raw::DiskMmapFile
            /// [`AsyncDiskMmapFile`]: fmmap::raw::AsyncDiskMmapFile
            /// [`MmapFile`]: struct.MmapFile.html
            /// [`AsyncMmapFile`]: struct.AsyncMmapFile.html
            /// [`std::fs::OpenOptions::create`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.create
            pub fn create(mut self, val: bool) -> Self {
                self.file_opts.create(val);
                self
            }

            /// Sets the option to create a new file, failing if it already exists. For details, please see [`std::fs::OpenOptions::create_new`]
            ///
            /// This field is ignored when opening [`DiskMmapFile`], [`AsyncDiskMmapFile`], [`MmapFile`] and [`AsyncMmapFile`].
            ///
            /// [`DiskMmapFile`]: fmmap::raw::DiskMmapFile
            /// [`AsyncDiskMmapFile`]: fmmap::raw::AsyncDiskMmapFile
            /// [`MmapFile`]: struct.MmapFile.html
            /// [`AsyncMmapFile`]: struct.AsyncMmapFile.html
            /// [`std::fs::OpenOptions::create_new`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.create_new
            pub fn create_new(mut self, val: bool) -> Self {
                self.file_opts.create_new(val);
                self
            }

            /// Sets the option for the append mode. For details, please see [`std::fs::OpenOptions::append`]
            ///
            /// This field is ignored when opening [`DiskMmapFile`], [`AsyncDiskMmapFile`], [`MmapFile`] and [`AsyncMmapFile`].
            ///
            /// [`DiskMmapFile`]: fmmap::raw::DiskMmapFile
            /// [`AsyncDiskMmapFile`]: fmmap::raw::AsyncDiskMmapFile
            /// [`MmapFile`]: struct.MmapFile.html
            /// [`AsyncMmapFile`]: struct.AsyncMmapFile.html
            /// [`std::fs::OpenOptions::append`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.append
            pub fn append(mut self, val: bool) -> Self {
                self.file_opts.append(val);
                self
            }

            /// Sets the option for truncating a previous file. For details, please see [`std::fs::OpenOptions::truncate`]
            ///
            /// This field is ignored when opening [`DiskMmapFile`], [`AsyncDiskMmapFile`], [`MmapFile`] and [`AsyncMmapFile`].
            ///
            /// [`DiskMmapFile`]: fmmap::raw::DiskMmapFile
            /// [`AsyncDiskMmapFile`]: fmmap::raw::AsyncDiskMmapFile
            /// [`MmapFile`]: struct.MmapFile.html
            /// [`AsyncMmapFile`]: struct.AsyncMmapFile.html
            /// [`std::fs::OpenOptions::truncate`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.truncate
            pub fn truncate(mut self, val: bool) -> Self {
                self.file_opts.truncate(val);
                self
            }

            /// Sets the mode bits that a new file will be created with. [Read more]
            ///
            /// [Read more]: https://doc.rust-lang.org/std/os/unix/fs/trait.OpenOptionsExt.html#tymethod.mode
            #[cfg(unix)]
            pub fn mode(mut self, mode: u32) -> Self {
                self.file_opts.mode(mode);
                self
            }

            /// Pass custom flags to the `flags` argument of `open`. [Read more]
            ///
            /// [Read more]: https://doc.rust-lang.org/std/os/unix/fs/trait.OpenOptionsExt.html#tymethod.mode
            #[cfg(unix)]
            pub fn custom_flags(mut self, flags: i32) -> Self {
                self.file_opts.custom_flags(flags);
                self
            }

            /// Overrides the `dwDesiredAccess` argument to the call to [`CreateFile`] with the specified value. [Read more]
            ///
            /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
            /// [Read more]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.security_qos_flags
            #[cfg(windows)]
            pub fn access_mode(mut self, access: u32) -> Self {
                self.file_opts.access_mode(access);
                self
            }

            /// Overrides the `dwShareMode` argument to the call to [`CreateFile`] with the specified value. [Read more]
            ///
            /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
            /// [Read more]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.security_qos_flags
            #[cfg(windows)]
            pub fn share_mode(mut self, val: u32) -> Self {
                self.file_opts.share_mode(val);
                self
            }

            /// Sets extra flags for the dwFileFlags argument to the
            /// call to [`CreateFile2`] to the specified value (or combines
            /// it with `attributes` and `security_qos_flags` to set the `dwFlagsAndAttributes` for [`CreateFile`]). [Read more]
            ///
            /// [`CreateFile2`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfile2
            /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
            /// [Read more]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.security_qos_flags
            #[cfg(windows)]
            pub fn custom_flags(mut self, flag: u32) -> Self {
                self.file_opts.custom_flags(flag);
                self
            }

            /// Overrides the `dwDesiredAccess` argument to the call to [`CreateFile`] with the specified value. [Read more]
            ///
            /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
            /// [Read more]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.security_qos_flags
            #[cfg(windows)]
            pub fn attributes(mut self, val: u32) -> Self {
                self.file_opts.attributes(val);
                self
            }

            /// Sets the `dwSecurityQosFlags` argument to the call to
            /// [`CreateFile2`] to the specified value (or combines it with `custom_flags`
            /// and `attributes` to set the `dwFlagsAndAttributes` for [`CreateFile`]). [Read more]
            ///
            /// [`CreateFile2`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfile2
            /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
            /// [Read more]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.security_qos_flags
            #[cfg(windows)]
            pub fn security_qos_flags(mut self, flags: u32) -> Self {
                self.file_opts.security_qos_flags(flags);
                self
            }
        }
    };
}

cfg_sync!(
    mod sync_impl;
    pub use sync_impl::Options;
);

cfg_async! {
    macro_rules! declare_and_impl_async_options {
        ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal) => {
            declare_and_impl_options!(AsyncOptions, OpenOptions);

            impl AsyncOptions {
                /// Create a new file and mmap this file with [`AsyncOptions`]
                ///
                /// # Example
                #[doc = "```rust"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                /// let mut file = AsyncOptions::new()
                ///     // truncate to 100
                ///     .max_size(100)
                #[doc = concat!(".create_mmap_file_mut(\"", $filename_prefix, "_create_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_create_with_options_test.txt\").unwrap());")]
                /// assert!(!file.is_empty());
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
                /// # })
                #[doc = "```"]
                ///
                /// [`AsyncOptions`]: struct.AsyncOptions.html
                pub async fn create_mmap_file_mut<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFileMut, Error> {
                    Ok(AsyncMmapFileMut::from(AsyncDiskMmapFileMut::create_with_options(path, self).await?))
                }

                /// Open a readable memory map backed by a file with [`Options`]
                ///
                /// # Example
                ///
                #[doc = "```rust"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFile, AsyncMmapFileExt};")]
                #[doc = concat!("use ", $path_str, "::fs::File;")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = File::create(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_open_with_options_test.txt\").unwrap());")]
                #[doc = concat!("# ", $path_str, "::io::AsyncWriteExt::write_all(&mut file, \"sanity text\".as_bytes()).await.unwrap();")]
                #[doc = concat!("# ", $path_str, "::io::AsyncWriteExt::write_all(&mut file, \"some data...\".as_bytes()).await.unwrap();")]
                /// # drop(file);
                ///
                /// // mmap the file
                /// let file = AsyncOptions::new()
                ///     // mmap content after the sanity text
                ///     .offset("sanity text".as_bytes().len() as u64)
                #[doc = concat!(".open_mmap_file(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                /// [`AsyncOptions`]: struct.AsyncOptions.html
                pub async fn open_mmap_file<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFile, Error> {
                    Ok(AsyncMmapFile::from(AsyncDiskMmapFile::open_with_options(path, self).await?))
                }

                /// Open a readable and executable memory map backed by a file with [`AsyncOptions`].
                ///
                /// # Examples
                ///
                #[doc = "```rust"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFile, AsyncMmapFileExt};")]
                #[doc = concat!("use ", $path_str, "::fs::File;")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = File::create(\"", $filename_prefix, "_open_exec_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_open_exec_with_options_test.txt\").unwrap());")]
                #[doc = concat!("# ", $path_str, "::io::AsyncWriteExt::write_all(&mut file, \"sanity text\".as_bytes()).await.unwrap();")]
                #[doc = concat!("# ", $path_str, "::io::AsyncWriteExt::write_all(&mut file, \"some data...\".as_bytes()).await.unwrap();")]
                /// # drop(file);
                ///
                /// // mmap the file
                /// let file = AsyncOptions::new()
                ///     // mmap content after the sanity text
                ///     .offset("sanity text".as_bytes().len() as u64)
                #[doc = concat!(".open_exec_mmap_file(\"", $filename_prefix, "_open_exec_with_options_test.txt\").await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                /// [`AsyncOptions`]: struct.AsyncOptions.html
                pub async fn open_exec_mmap_file<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFile, Error> {
                    Ok(AsyncMmapFile::from(AsyncDiskMmapFile::open_exec_with_options(path, self).await?))
                }

                /// Open or Create(if not exists) a file and mmap this file with [`AsyncOptions`].
                ///
                /// # Examples
                ///
                /// File already exists
                ///
                /// ```rust
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
                #[doc = concat!("use ", $path_str, "::fs::File;")]
                /// use std::io::SeekFrom;
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = File::create(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_with_options_test.txt\").unwrap());")]
                #[doc = concat!("# ", $path_str, "::io::AsyncWriteExt::write_all(&mut file, \"sanity text\".as_bytes()).await.unwrap();")]
                #[doc = concat!("# ", $path_str, "::io::AsyncWriteExt::write_all(&mut file, \"some data...\".as_bytes()).await.unwrap();")]
                /// # drop(file);
                ///
                /// let mut file = AsyncOptions::new()
                ///     // allow read
                ///     .read(true)
                ///     // allow write
                ///     .write(true)
                ///     // allow append
                ///     .append(true)
                ///     // truncate to 100
                ///     .max_size(100)
                ///     // mmap content after the sanity text
                ///     .offset("sanity text".as_bytes().len() as u64)
                #[doc = concat!(".open_mmap_file_mut(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
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
                #[doc = concat!("let mut file = File::open(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
                /// // skip the sanity text
                #[doc = concat!($path_str, "::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start(\"sanity text\".as_bytes().len() as u64)).await.unwrap();")]
                #[doc = concat!($path_str, "::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();")]
                /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                /// File does not exists
                ///
                /// ```no_run
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
                #[doc = concat!("use ", $path_str, "::fs::File;")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                /// // mmap the file with options
                /// let mut file = AsyncOptions::new()
                ///     // allow read
                ///     .read(true)
                ///     // allow write
                ///     .write(true)
                ///     // allow append
                ///     .append(true)
                ///     // truncate to 100
                ///     .max_size(100)
                #[doc = concat!(".open_mmap_file_mut(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
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
                #[doc = concat!("let mut file = File::open(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!($path_str, "::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();")]
                /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                /// [`AsyncOptions`]: struct.AsyncOptions.html
                pub async fn open_mmap_file_mut<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFileMut, Error> {
                    Ok(AsyncMmapFileMut::from(AsyncDiskMmapFileMut::open_with_options(path, self).await?))
                }

                /// Open an existing file and mmap this file with [`AsyncOptions`]
                ///
                /// # Example
                ///
                /// ```rust
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
                #[doc = concat!("use ", $path_str, "::fs::File;")]
                /// use std::io::SeekFrom;
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                /// // create a temp file
                #[doc = concat!("let mut file = File::create(\"", $filename_prefix, "_open_existing_test_with_options.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_existing_test_with_options.txt\").unwrap());")]
                #[doc = concat!("# ", $path_str, "::io::AsyncWriteExt::write_all(&mut file, \"sanity text\".as_bytes()).await.unwrap();")]
                #[doc = concat!("# ", $path_str, "::io::AsyncWriteExt::write_all(&mut file, \"some data...\".as_bytes()).await.unwrap();")]
                /// drop(file);
                ///
                /// // mmap the file
                /// let mut file = AsyncOptions::new()
                ///     // truncate to 100
                ///     .max_size(100)
                ///     // mmap content after the sanity text
                ///     .offset("sanity text".as_bytes().len() as u64)
                #[doc = concat!(".open_exist_mmap_file_mut(\"", $filename_prefix, "_open_existing_test_with_options.txt\").await.unwrap();")]
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
                #[doc = concat!("let mut file = File::open(\"", $filename_prefix, "_open_existing_test_with_options.txt\").await.unwrap();")]
                /// let mut buf = vec![0; "some modified data...".len()];
                /// // skip the sanity text
                #[doc = concat!($path_str, "::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start(\"sanity text\".as_bytes().len() as u64)).await.unwrap();")]
                #[doc = concat!($path_str, "::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();")]
                /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                /// [`AsyncOptions`]: struct.AsyncOptions.html
                pub async fn open_exist_mmap_file_mut<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFileMut, Error> {
                    Ok(AsyncMmapFileMut::from(AsyncDiskMmapFileMut::open_exist_with_options(path, self).await?))
                }

                /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file) with [`AsyncOptions`].
                /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
                ///
                /// # Examples
                ///
                #[doc = "```rust"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
                #[doc = concat!("use ", $path_str, "::fs::File;")]
                /// use std::io::SeekFrom;
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                /// // create a temp file
                #[doc = concat!("let mut file = File::create(\"", $filename_prefix, "_open_cow_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!("#  defer!(std::fs::remove_file(\"", $filename_prefix, "_open_cow_with_options_test.txt\").unwrap());")]
                #[doc = concat!($path_str, "::io::AsyncWriteExt::write_all(&mut file, \"sanity text\".as_bytes()).await.unwrap();")]
                #[doc = concat!($path_str, "::io::AsyncWriteExt::write_all(&mut file, \"some data...\".as_bytes()).await.unwrap();")]
                /// drop(file);
                ///
                /// // mmap the file
                /// let mut file = AsyncOptions::new()
                ///     // mmap content after the sanity text
                ///     .offset("sanity text".as_bytes().len() as u64)
                #[doc = concat!(".open_cow_mmap_file_mut(\"", $filename_prefix, "_open_cow_with_options_test.txt\").await.unwrap();")]
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
                #[doc = concat!("let mut file = File::open(\"", $filename_prefix, "_open_cow_with_options_test.txt\").await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// // skip the sanity text
                #[doc = concat!($path_str, "::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start(\"sanity text\".as_bytes().len() as u64)).await.unwrap();")]
                #[doc = concat!($path_str, "::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();")]
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                ///
                /// [`AsyncOptions`]: struct.AsyncOptions.html
                pub async fn open_cow_mmap_file_mut<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFileMut, Error> {
                    Ok(AsyncMmapFileMut::from(AsyncDiskMmapFileMut::open_cow_with_options(path, self).await?))
                }
            }
        };
    }
}

cfg_async_std!(
    pub mod async_std_impl;
);

cfg_smol!(
    pub mod smol_impl;
);

cfg_tokio!(
    pub mod tokio_impl;
);

