macro_rules! read_impl {
    ($this:ident, $offset: tt, $typ:tt::$conv:tt) => {{
        const SIZE: usize = mem::size_of::<$typ>();
        // try to convert directly from the bytes
        // this Option<ret> trick is to avoid keeping a borrow on self
        // when advance() is called (mut borrow) and to call bytes() only once
        let mut buf = [0; SIZE];
        $this
            .read_exact(&mut buf, $offset)
            .map(|_| unsafe { $typ::$conv(*(&buf as *const _ as *const [_; SIZE])) })
    }};
}

macro_rules! impl_from {
    ($outer: ident, $enum_inner: ident, [$($inner: ident), +$(,)?]) => {
        $(
        impl From<$inner> for $outer {
            fn from(file: $inner) -> Self {
                $outer{ inner: <$enum_inner>::from(file) }
            }
        }
        )*
    };
}

macro_rules! impl_from_mut {
    ($outer: ident, $enum_inner: ident, [$($inner: ident), +$(,)?]) => {
        $(
        impl From<$inner> for $outer {
            fn from(file: $inner) -> Self {
                $outer{
                    inner: <$enum_inner>::from(file),
                    remove_on_drop: false,
                    deleted: false,
                }
            }
        }
        )*
    };
}

macro_rules! impl_drop {
    ($name: ident, $inner: ident, $empty: ident) => {
        impl Drop for $name {
            fn drop(&mut self) {
                if self.remove_on_drop && !self.deleted {
                    let empty = <$inner>::Empty(<$empty>::default());
                    // swap the inner to empty
                    let inner = mem::replace(&mut self.inner, empty);
                    // do remove and ignore the result
                    let path = inner.path_buf();
                    drop(inner);
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    };
}

macro_rules! impl_flush {
    () => {
        fn flush(&self) -> Result<()> {
            self.inner.flush()
        }

        fn flush_async(&self) -> Result<()> {
            self.inner.flush_async()
        }

        fn flush_range(&self, offset: usize, len: usize) -> Result<()> {
            self.inner.flush_range(offset, len)
        }

        fn flush_async_range(&self, offset: usize, len: usize) -> Result<()> {
            self.inner.flush_async_range(offset, len)
        }
    };
}

macro_rules! impl_file_lock {
    () => {
        #[inline]
        fn lock_exclusive(&self) -> crate::error::Result<()> {
            self.inner.lock_exclusive()
        }

        #[inline]
        fn lock_shared(&self) -> crate::error::Result<()> {
            self.inner.lock_shared()
        }

        #[inline]
        fn try_lock_exclusive(&self) -> crate::error::Result<()> {
            self.inner.try_lock_exclusive()
        }

        #[inline]
        fn try_lock_shared(&self) -> crate::error::Result<()> {
            self.inner.try_lock_shared()
        }

        #[inline]
        fn unlock(&self) -> crate::error::Result<()> {
            self.inner.unlock()
        }
    };
}

macro_rules! impl_constructor_for_memory_mmap_file {
    ($memory_base: ident, $name: ident, $name_str: literal, $path_str: literal) => {
        use bytes::Bytes;

        impl $name {
            #[doc = concat!("Create a in-memory ", $name_str)]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = "use bytes::{BufMut, BytesMut};"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let mut data = BytesMut::with_capacity(100);"]
            #[doc = "data.put_slice(\"some data...\".as_bytes());"]
            #[doc = concat!($name_str, "::memory(\"foo.mem\", data.freeze());")]
            #[doc = "```"]
            pub fn memory<P: AsRef<Path>>(path: P, data: Bytes) -> Self {
                Self::from(<$memory_base>::new(path, data))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from Vec")]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data = (0..=255u8).collect::<Vec<_>>();"]
            #[doc = concat!($name_str, "::memory_from_vec(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn memory_from_vec<P: AsRef<Path>>(path: P, src: Vec<u8>) -> Self {
                Self::from(<$memory_base>::from_vec(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from String")]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::memory_from_string(\"foo.mem\", data.to_string());")]
            #[doc = "```"]
            pub fn memory_from_string<P: AsRef<Path>>(path: P, src: String) -> Self {
                Self::from(<$memory_base>::from_string(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from static slice")]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = "use bytes::Bytes;"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static [u8] = \"some data...\".as_bytes();"]
            #[doc = concat!($name_str, "::memory_from_slice(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn memory_from_slice<P: AsRef<Path>>(path: P, src: &'static [u8]) -> Self {
                 Self::from(<$memory_base>::from_slice(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from static str")]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = "use bytes::Bytes;"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::memory_from_str(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn memory_from_str<P: AsRef<Path>>(path: P, src: &'static str) -> Self {
                Self::from(<$memory_base>::from_str(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " by copy from slice")]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!($name_str, "::memory_copy_from_slice(\"foo.mem\", \"some data...\".as_bytes());")]
            #[doc = "```"]
            pub fn memory_copy_from_slice<P: AsRef<Path>>(path: P, src: &[u8]) -> Self {
                Self::from(<$memory_base>::copy_from_slice(path, src))
            }
        }
    };
}

macro_rules! impl_constructor_for_memory_mmap_file_mut {
    ($memory_base: ident, $name: ident, $name_str: literal, $path_str: literal) => {
        impl $name {
            #[doc = concat!("Create a in-memory ", $name_str)]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!($name_str, "::memory(\"foo.mem\");")]
            #[doc = "```"]
            pub fn memory<P: AsRef<Path>>(path: P) -> Self {
                Self::from(<$memory_base>::new(path))
            }

            #[doc = concat!("Create a in-memory ", $name_str, "with capacity")]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!($name_str, "::memory_with_capacity(\"foo.mem\", 1000);")]
            #[doc = "```"]
            pub fn memory_with_capacity<P: AsRef<Path>>(path: P, cap: usize) -> Self {
                Self::from(<$memory_base>::with_capacity(path, cap))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from Vec")]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data = (0..=255u8).collect::<Vec<_>>();"]
            #[doc = concat!($name_str, "::memory_from_vec(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn memory_from_vec<P: AsRef<Path>>(path: P, src: Vec<u8>) -> Self {
                Self::from(<$memory_base>::from_vec(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from String")]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::memory_from_string(\"foo.mem\", data.to_string());")]
            #[doc = "```"]
            pub fn memory_from_string<P: AsRef<Path>>(path: P, src: String) -> Self {
                Self::from(<$memory_base>::from_string(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from static str")]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = "use bytes::Bytes;"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::memory_from_str(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn memory_from_str<P: AsRef<Path>>(path: P, src: &'static str) -> Self {
                Self::from(<$memory_base>::from_str(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " by from slice")]
            #[doc = "# Examples"]
            #[doc = "```ignore"]
            #[doc = concat!("use fmmap::", $path_str, "::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!($name_str, "::memory_from_slice(\"foo.mem\", \"some data...\".as_bytes());")]
            #[doc = "```"]
            pub fn memory_from_slice<P: AsRef<Path>>(path: P, src: &[u8]) -> Self {
                Self::from(<$memory_base>::from_slice(path, src))
            }
        }
    };
}

cfg_sync! {
    macro_rules! impl_mmap_file_ext {
        ($name: ident) => {
            impl MmapFileExt for $name {
                #[inline]
                fn len(&self) -> usize {
                    self.inner.len()
                }

                #[inline]
                fn as_slice(&self) -> &[u8] {
                    self.inner.as_slice()
                }

                #[inline]
                fn path(&self) -> &Path {
                    self.inner.path()
                }

                #[inline]
                fn is_exec(&self) -> bool {
                    self.inner.is_exec()
                }

                #[inline]
                fn metadata(&self) -> Result<MetaData> {
                    self.inner.metadata()
                }

                impl_file_lock!();
            }
        };
    }

    mod sync_impl;
    pub use sync_impl::{MmapFileExt, MmapFileMutExt, MmapFile, MmapFileMut};
}

cfg_async! {
    macro_rules! impl_async_mmap_file_ext {
        ($name: ident) => {
            #[async_trait]
            impl AsyncMmapFileExt for $name {
                #[inline]
                fn len(&self) -> usize {
                    self.inner.len()
                }

                #[inline]
                fn as_slice(&self) -> &[u8] {
                    self.inner.as_slice()
                }

                #[inline]
                fn path(&self) -> &Path {
                    self.inner.path()
                }

                #[inline]
                fn is_exec(&self) -> bool {
                    self.inner.is_exec()
                }

                #[inline]
                async fn metadata(&self) -> Result<MetaData> {
                    self.inner.metadata().await
                }

                impl_file_lock!();
            }
        };
    }

    macro_rules! impl_async_mmap_file_mut_ext {
        ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal) => {
            #[async_trait]
            impl AsyncMmapFileMutExt for AsyncMmapFileMut {
                #[inline]
                fn as_mut_slice(&mut self) -> &mut [u8] {
                    self.inner.as_mut_slice()
                }

                #[inline]
                fn is_cow(&self) -> bool {
                    self.inner.is_cow()
                }

                impl_flush!();

                #[inline]
                async fn truncate(&mut self, max_sz: u64) -> Result<()> {
                    self.inner.truncate(max_sz).await
                }

                /// Remove the underlying file
                ///
                /// # Example
                ///
                /// ```ignore
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
                #[doc = concat!("use ", $path_str, "::fs::File;")]
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_remove_test.txt\").await.unwrap();")]
                ///
                /// file.truncate(12).await;
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
                ///
                /// file.remove().await.unwrap();
                ///
                #[doc = concat!("let err = File::open(\"", $filename_prefix, "_remove_test.txt\").await;")]
                /// assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
                /// # })
                /// ```
                async fn remove(mut self) -> Result<()> {
                    let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
                    // swap the inner to empty
                    let inner = mem::replace(&mut self.inner, empty);
                    if !self.remove_on_drop {
                        // do remove
                        inner.remove().await?;
                        self.deleted = true;
                    }
                    Ok(())
                }

                /// Close and truncate the underlying file
                ///
                /// # Examples
                ///
                /// ```ignore
                #[doc = concat!("use fmmap::{MetaDataExt,", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt}};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_close_with_truncate_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_close_with_truncate_test.txt\").unwrap());")]
                /// file.truncate(100).await;
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
                ///
                /// file.close_with_truncate(50).await.unwrap();
                ///
                #[doc = concat!("let file = AsyncMmapFileMut::open(\"", $filename_prefix, "_close_with_truncate_test.txt\").await.unwrap();")]
                /// let meta = file.metadata().await.unwrap();
                /// assert_eq!(meta.len(), 50);
                /// # })
                /// ```
                async fn close_with_truncate(mut self, max_sz: i64) -> Result<()> {
                    let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
                    // swap the inner to empty
                    let inner = mem::replace(&mut self.inner, empty);
                    inner.close_with_truncate(max_sz).await
                }
            }
        };
    }

    macro_rules! declare_async_mmap_file_ext {
        ($disk_file_mut: ty, $opts: ty, $reader: ty) => {
            /// Utility methods to [`AsyncMmapFile`]
            ///
            /// [`AsyncMmapFile`]: structs.AsyncMmapFile.html
            #[async_trait]
            #[enum_dispatch]
            pub trait AsyncMmapFileExt {
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
                    &self.as_slice()[offset..offset+sz]
                }

                /// bytes returns data starting from offset off of size sz.
                ///
                /// # Errors
                /// If there's not enough data, it would return
                /// `Err(Error::from(ErrorKind::EOF))`.
                fn bytes(&self, offset: usize, sz: usize) -> Result<&[u8]> {
                    let buf = self.as_slice();
                    if buf.len() < offset + sz {
                        Err(Error::from(ErrorKind::EOF))
                    } else {
                        Ok(&buf[offset..offset+sz])
                    }
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

                /// Whether the mmap is executable
                fn is_exec(&self) -> bool;

                /// Returns the metadata of file metadata
                ///
                /// Metadata information about a file.
                /// This structure is returned from the metadata or
                /// symlink_metadata function or method and represents
                /// known metadata about a file such as its permissions, size, modification times, etc
                async fn metadata(&self) -> Result<MetaData>;

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
                #[inline]
                async fn write_all_to_new_file<P: AsRef<Path> + Send + Sync>(&self, new_file_path: P) -> Result<()> {
                    let buf = self.as_slice();
                    let opts = <$opts>::new().max_size(buf.len() as u64);

                    let mut mmap = <$disk_file_mut>::create_with_options(new_file_path, opts).await?;
                    mmap.writer(0)?.write_all(buf).await?;
                    mmap.flush()
                }

                /// Write a range of content of the mmap file to new file.
                #[inline]
                async fn write_range_to_new_file<P: AsRef<Path> + Send + Sync>(&self, new_file_path: P, offset: usize, len: usize) -> Result<()> {
                    let buf = self.as_slice();
                    if buf.len() < offset + len {
                        return Err(Error::from(ErrorKind::EOF));
                    }
                    let opts = <$opts>::new().max_size(len as u64);

                    let mut mmap = <$disk_file_mut>::create_with_options(new_file_path, opts).await?;
                    mmap.writer(0)?.write_all(&buf[offset..offset + len]).await?;
                    mmap.flush()
                }

                /// Returns a [`AsyncMmapFileReader`] which helps read data from mmap like a normal File.
                ///
                /// # Errors
                /// If there's not enough data, it would return
                ///  `Err(Error::from(ErrorKind::EOF))`.
                ///
                /// [`AsyncMmapFileReader`]: structs.AsyncMmapFileReader.html
                fn reader(&self, offset: usize) -> Result<$reader> {
                    let buf = self.as_slice();
                    if buf.len() < offset {
                        Err(Error::from(ErrorKind::EOF))
                    } else {
                        Ok(<$reader>::new(Cursor::new(&buf[offset..]), offset, buf.len() - offset))
                    }
                }

                /// Returns a [`AsyncMmapFileReader`] base on the given `offset` and `len`, which helps read data from mmap like a normal File.
                ///
                /// # Errors
                /// If there's not enough data, it would return
                ///  `Err(Error::from(ErrorKind::EOF))`.
                ///
                /// [`AsyncMmapFileReader`]: structs.AsyncMmapFileReader.html
                fn range_reader(&self, offset: usize, len: usize) -> Result<$reader> {
                    let buf = self.as_slice();
                    if buf.len() < offset + len {
                        Err(Error::from(ErrorKind::EOF))
                    } else {
                        Ok(<$reader>::new(Cursor::new(&buf[offset.. offset + len]), offset, len))
                    }
                }

                /// Locks the file for shared usage, blocking if the file is currently locked exclusively.
                ///
                /// # Notes
                /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
                fn lock_exclusive(&self) -> Result<()>;

                /// Locks the file for exclusive usage, blocking if the file is currently locked.
                ///
                /// # Notes
                /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
                fn lock_shared(&self) -> Result<()>;

                /// Locks the file for shared usage, or returns a an error if the file is currently locked (see lock_contended_error).
                ///
                /// # Notes
                /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
                fn try_lock_exclusive(&self) -> Result<()>;

                /// Locks the file for shared usage, or returns a an error if the file is currently locked (see lock_contended_error).Locks the file for shared usage, or returns a an error if the file is currently locked (see lock_contended_error).
                ///
                /// # Notes
                /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
                fn try_lock_shared(&self) -> Result<()>;

                /// Unlocks the file.
                ///
                /// # Notes
                /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
                fn unlock(&self) -> Result<()>;

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
                            dst.copy_from_slice(&buf[offset..offset + remaining]);
                            remaining
                        }
                    }
                }

                /// Read the exact number of bytes required to fill buf.
                fn read_exact(&self, dst: &mut [u8], offset: usize) -> Result<()> {
                    let buf = self.as_slice();
                    let remaining = buf.len().checked_sub(offset);
                    match remaining {
                        None => Err(Error::from(ErrorKind::EOF)),
                        Some(remaining) => {
                            let dst_len = dst.len();
                            if remaining < dst_len {
                                Err(Error::from(ErrorKind::EOF))
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
                        None => Err(Error::from(ErrorKind::EOF)),
                        Some(remaining) => {
                            if remaining < 1 {
                                Err(Error::from(ErrorKind::EOF))
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
                        None => Err(Error::from(ErrorKind::EOF)),
                        Some(remaining) => {
                            if remaining < 1 {
                                Err(Error::from(ErrorKind::EOF))
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
        };
    }

    macro_rules! declare_async_mmap_file_mut_ext {
        ($writer: ty) => {
            /// Utility methods to [`AsyncMmapFileMut`]
            ///
            /// [`AsyncMmapFileMut`]: structs.AsyncMmapFileMut.html
            #[async_trait]
            #[enum_dispatch]
            pub trait AsyncMmapFileMutExt {
                /// Returns the mutable underlying slice of the mmap
                fn as_mut_slice(&mut self) -> &mut [u8];

                /// slice_mut returns mutable data starting from offset off of size sz.
                ///
                /// # Panics
                /// If there's not enough data, it would
                /// panic.
                fn slice_mut(&mut self, offset: usize, sz: usize) -> &mut [u8] {
                    &mut self.as_mut_slice()[offset..offset+sz]
                }

                /// Whether mmap is copy on write
                fn is_cow(&self) -> bool;

                /// bytes_mut returns mutable data starting from offset off of size sz.
                ///
                /// # Errors
                /// If there's not enough data, it would return
                /// `Err(Error::from(ErrorKind::EOF))`.
                fn bytes_mut(&mut self, offset: usize, sz: usize) -> Result<&mut [u8]> {
                    let buf = self.as_mut_slice();
                    if buf.len() < offset + sz {
                        Err(Error::from(ErrorKind::EOF))
                    } else {
                        Ok(&mut buf[offset..offset+sz])
                    }
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
                async fn truncate(&mut self, max_sz: u64) -> Result<()>;

                /// Remove the underlying file
                async fn remove(self) -> Result<()>;

                /// Close and truncate the underlying file
                async fn close_with_truncate(self, max_sz: i64) -> Result<()>;

                /// Returns a [`AsyncMmapFileWriter`] base on the given `offset`, which helps read or write data from mmap like a normal File.
                ///
                /// # Notes
                /// If you use a writer to write data to mmap, there is no guarantee all
                /// data will be durably stored. So you need to call [`flush`]/[`flush_range`]/[`flush_async`]/[`flush_async_range`] in [`AsyncMmapFileMutExt`]
                /// to guarantee all data will be durably stored.
                ///
                /// # Errors
                /// If there's not enough data, it would return
                ///  `Err(Error::from(ErrorKind::EOF))`.
                ///
                /// [`flush`]: traits.MmapFileMutExt.html#methods.flush
                /// [`flush_range`]: traits.MmapFileMutExt.html#methods.flush_range
                /// [`flush_async`]: traits.MmapFileMutExt.html#methods.flush_async
                /// [`flush_async_range`]: traits.MmapFileMutExt.html#methods.flush_async_range
                /// [`MmapFileWriter`]: structs.MmapFileWriter.html
                fn writer(&mut self, offset: usize) -> Result<$writer> {
                    let buf = self.as_mut_slice();
                    let buf_len = buf.len();
                    if buf_len < offset {
                        Err(Error::from(ErrorKind::EOF))
                    } else {
                        Ok(<$writer>::new(Cursor::new(&mut buf[offset..]), offset, buf_len - offset))
                    }
                }

                /// Returns a [`AsyncMmapFileWriter`] base on the given `offset` and `len`, which helps read or write data from mmap like a normal File.
                ///
                /// # Notes
                /// If you use a writer to write data to mmap, there is no guarantee all
                /// data will be durably stored. So you need to call [`flush`]/[`flush_range`]/[`flush_async`]/[`flush_async_range`] in [`MmapFileMutExt`]
                /// to guarantee all data will be durably stored.
                ///
                /// # Errors
                /// If there's not enough data, it would return
                ///  `Err(Error::from(ErrorKind::EOF))`.
                ///
                /// [`flush`]: traits.AsyncMmapFileMutExt.html#methods.flush
                /// [`flush_range`]: traits.AsyncMmapFileMutExt.html#methods.flush_range
                /// [`flush_async`]: traits.AsyncMmapFileMutExt.html#methods.flush_async
                /// [`flush_async_range`]: traits.AsyncMmapFileMutExt.html#methods.flush_async_range
                /// [`AsyncMmapFileWriter`]: structs.AsyncMmapFileWriter.html
                fn range_writer(&mut self, offset: usize, len: usize) -> Result<$writer> {
                    let buf = self.as_mut_slice();
                    if buf.len() < offset + len {
                        Err(Error::from(ErrorKind::EOF))
                    } else {
                        Ok(<$writer>::new(
                            Cursor::new(&mut buf[offset..offset + len]), offset, len))
                    }
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
                        None => Err(Error::from(ErrorKind::EOF)),
                        Some(remaining) => {
                            let src_len = src.len();
                            if remaining < src_len {
                                Err(Error::from(ErrorKind::EOF))
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
        };
    }

    macro_rules! declare_and_impl_inners {
        () => {
            enum AsyncMmapFileInner {
                Empty(AsyncEmptyMmapFile),
                Memory(AsyncMemoryMmapFile),
                Disk(AsyncDiskMmapFile)
            }

            impl From<AsyncEmptyMmapFile> for AsyncMmapFileInner {
                fn from(v: AsyncEmptyMmapFile) -> AsyncMmapFileInner {
                    AsyncMmapFileInner::Empty(v)
                }
            }
            impl From<AsyncMemoryMmapFile> for AsyncMmapFileInner {
                fn from(v: AsyncMemoryMmapFile) -> AsyncMmapFileInner {
                    AsyncMmapFileInner::Memory(v)
                }
            }
            impl From<AsyncDiskMmapFile> for AsyncMmapFileInner {
                fn from(v: AsyncDiskMmapFile) -> AsyncMmapFileInner {
                    AsyncMmapFileInner::Disk(v)
                }
            }

            #[async_trait]
            impl AsyncMmapFileExt for AsyncMmapFileInner {
                #[inline]
                fn len(&self) -> usize {
                    match self {
                        AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::len(inner),
                        AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::len(inner),
                        AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::len(inner),
                    }
                }

                #[inline]
                fn as_slice(&self) -> &[u8] {
                    match self {
                        AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::as_slice(inner),
                        AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::as_slice(inner),
                        AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::as_slice(inner),
                    }
                }

                #[inline]
                fn path(&self) -> &Path {
                    match self {
                        AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::path(inner),
                        AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::path(inner),
                        AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::path(inner),
                    }
                }

                #[inline]
                fn is_exec(&self) -> bool {
                    match self {
                        AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::is_exec(inner),
                        AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::is_exec(inner),
                        AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::is_exec(inner),
                    }
                }

                #[inline]
                async fn metadata(&self) -> Result<MetaData> {
                    match self {
                        AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::metadata(inner).await,
                        AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::metadata(inner).await,
                        AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::metadata(inner).await,
                    }
                }

                #[inline]
                fn lock_exclusive(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::lock_exclusive(inner),
                        AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::lock_exclusive(inner),
                        AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::lock_exclusive(inner),
                    }
                }

                #[inline]
                fn lock_shared(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::lock_shared(inner),
                        AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::lock_shared(inner),
                        AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::lock_shared(inner),
                    }
                }

                #[inline]
                fn try_lock_exclusive(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::try_lock_exclusive(inner),
                        AsyncMmapFileInner::Memory(inner) => {
                            AsyncMmapFileExt::try_lock_exclusive(inner)
                        }
                        AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::try_lock_exclusive(inner),
                    }
                }

                #[inline]
                fn try_lock_shared(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::try_lock_shared(inner),
                        AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::try_lock_shared(inner),
                        AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::try_lock_shared(inner),
                    }
                }

                #[inline]
                fn unlock(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileInner::Empty(inner) => AsyncMmapFileExt::unlock(inner),
                        AsyncMmapFileInner::Memory(inner) => AsyncMmapFileExt::unlock(inner),
                        AsyncMmapFileInner::Disk(inner) => AsyncMmapFileExt::unlock(inner),
                    }
                }
            }

            enum AsyncMmapFileMutInner {
                Empty(AsyncEmptyMmapFile),
                Memory(AsyncMemoryMmapFileMut),
                Disk(AsyncDiskMmapFileMut)
            }

            impl From<AsyncEmptyMmapFile> for AsyncMmapFileMutInner {
                fn from(v: AsyncEmptyMmapFile) -> AsyncMmapFileMutInner {
                    AsyncMmapFileMutInner::Empty(v)
                }
            }
            impl From<AsyncMemoryMmapFileMut> for AsyncMmapFileMutInner {
                fn from(v: AsyncMemoryMmapFileMut) -> AsyncMmapFileMutInner {
                    AsyncMmapFileMutInner::Memory(v)
                }
            }
            impl From<AsyncDiskMmapFileMut> for AsyncMmapFileMutInner {
                fn from(v: AsyncDiskMmapFileMut) -> AsyncMmapFileMutInner {
                    AsyncMmapFileMutInner::Disk(v)
                }
            }

            #[async_trait]
            impl AsyncMmapFileExt for AsyncMmapFileMutInner {
                #[inline]
                fn len(&self) -> usize {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::len(inner),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::len(inner),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::len(inner),
                    }
                }

                #[inline]
                fn as_slice(&self) -> &[u8] {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::as_slice(inner),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::as_slice(inner),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::as_slice(inner),
                    }
                }

                #[inline]
                fn path(&self) -> &Path {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::path(inner),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::path(inner),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::path(inner),
                    }
                }

                #[inline]
                fn is_exec(&self) -> bool {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::is_exec(inner),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::is_exec(inner),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::is_exec(inner),
                    }
                }

                #[inline]
                async fn metadata(&self) -> Result<MetaData> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::metadata(inner).await,
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::metadata(inner).await,
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::metadata(inner).await,
                    }
                }

                #[inline]
                fn lock_exclusive(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::lock_exclusive(inner),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::lock_exclusive(inner),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::lock_exclusive(inner),
                    }
                }

                #[inline]
                fn lock_shared(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::lock_shared(inner),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::lock_shared(inner),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::lock_shared(inner),
                    }
                }

                #[inline]
                fn try_lock_exclusive(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::try_lock_exclusive(inner),
                        AsyncMmapFileMutInner::Memory(inner) => {
                            AsyncMmapFileExt::try_lock_exclusive(inner)
                        }
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::try_lock_exclusive(inner),
                    }
                }

                #[inline]
                fn try_lock_shared(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::try_lock_shared(inner),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::try_lock_shared(inner),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::try_lock_shared(inner),
                    }
                }

                #[inline]
                fn unlock(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileExt::unlock(inner),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileExt::unlock(inner),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileExt::unlock(inner),
                    }
                }
            }

            #[async_trait]
            impl AsyncMmapFileMutExt for AsyncMmapFileMutInner {
                #[inline]
                fn as_mut_slice(&mut self) -> &mut [u8] {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::as_mut_slice(inner),
                        AsyncMmapFileMutInner::Memory(inner) => {
                            AsyncMmapFileMutExt::as_mut_slice(inner)
                        }
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::as_mut_slice(inner),
                    }
                }

                #[inline]
                fn is_cow(&self) -> bool {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::is_cow(inner),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::is_cow(inner),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::is_cow(inner),
                    }
                }

                #[inline]
                fn flush(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::flush(inner),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::flush(inner),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::flush(inner),
                    }
                }

                #[inline]
                fn flush_async(&self) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::flush_async(inner),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::flush_async(inner),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::flush_async(inner),
                    }
                }

                #[inline]
                fn flush_range(&self, offset: usize, len: usize) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::flush_range(
                            inner,
                            offset,
                            len,
                        ),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::flush_range(
                            inner,
                            offset,
                            len,
                        ),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::flush_range(
                            inner,
                            offset,
                            len,
                        ),
                    }
                }

                #[inline]
                fn flush_async_range(&self, offset: usize, len: usize) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::flush_async_range(
                            inner,
                            offset,
                            len,
                        ),
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::flush_async_range(
                            inner,
                            offset,
                            len,
                        ),
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::flush_async_range(
                            inner,
                            offset,
                            len,
                        ),
                    }
                }

                async fn truncate(&mut self, max_sz: u64) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => {
                            AsyncMmapFileMutExt::truncate(inner, max_sz).await
                        }
                        AsyncMmapFileMutInner::Memory(inner) => {
                            AsyncMmapFileMutExt::truncate(inner, max_sz).await
                        }
                        AsyncMmapFileMutInner::Disk(inner) => {
                            AsyncMmapFileMutExt::truncate(inner, max_sz).await
                        }
                    }
                }

                async fn remove(self) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => AsyncMmapFileMutExt::remove(inner).await,
                        AsyncMmapFileMutInner::Memory(inner) => AsyncMmapFileMutExt::remove(inner).await,
                        AsyncMmapFileMutInner::Disk(inner) => AsyncMmapFileMutExt::remove(inner).await,
                    }
                }

                async fn close_with_truncate(self, max_sz: i64) -> Result<()> {
                    match self {
                        AsyncMmapFileMutInner::Empty(inner) => {
                            AsyncMmapFileMutExt::close_with_truncate(inner, max_sz).await
                        }
                        AsyncMmapFileMutInner::Memory(inner) => {
                            AsyncMmapFileMutExt::close_with_truncate(inner, max_sz).await
                        }
                        AsyncMmapFileMutInner::Disk(inner) => {
                            AsyncMmapFileMutExt::close_with_truncate(inner, max_sz).await
                        }
                    }
                }
            }

        };
    }


    macro_rules! declare_and_impl_async_mmap_file {
        ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal) => {
            /// A read-only memory map file.
            ///
            /// There is 3 status of this struct:
            /// - __Disk__: mmap to a real file
            /// - __Memory__: use [`Bytes`] to mock a mmap, which is useful for test and in-memory storage engine
            /// - __Empty__: a state represents null mmap, which is helpful for drop, close the `AsyncMmapFile`. This state cannot be constructed directly.
            ///
            /// [`Bytes`]: https://docs.rs/bytes/1.1.0/bytes/struct.Bytes.html
            #[repr(transparent)]
            pub struct AsyncMmapFile {
                inner: AsyncMmapFileInner
            }

            impl_from!(AsyncMmapFile, AsyncMmapFileInner, [AsyncEmptyMmapFile, AsyncMemoryMmapFile, AsyncDiskMmapFile]);

            impl_async_mmap_file_ext!(AsyncMmapFile);

            impl AsyncMmapFile {
                /// Open a readable memory map backed by a file
                ///
                /// # Examples
                ///
                #[doc = "```ignore"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFile, AsyncMmapFileExt};")]
                #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
                #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_open_test.txt\").unwrap());")]
                /// # file.truncate(12).await.unwrap();
                /// # file.write_all("some data...".as_bytes(), 0).unwrap();
                /// # file.flush().unwrap();
                /// # drop(file);
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncMmapFile::open(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                #[doc = "```"]
                pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFile::open(path).await?))
                }

                /// Open a readable memory map backed by a file with [`Options`]
                ///
                /// # Examples
                ///
                #[doc = "```ignore"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFile, AsyncMmapFileExt};")]
                #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_open_with_options_test.txt\").unwrap());")]
                /// # file.truncate(23).await.unwrap();
                /// # file.write_all("sanity text".as_bytes(), 0).unwrap();
                /// # file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                /// # file.flush().unwrap();
                /// # drop(file);
                ///
                /// // mmap the file
                /// let opts = AsyncOptions::new()
                ///     // mmap content after the sanity text
                ///     .offset("sanity text".as_bytes().len() as u64);
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncMmapFile::open_with_options(\"", $filename_prefix, "_open_with_options_test.txt\", opts).await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
                pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFile::open_with_options(path, opts).await?))
                }

                /// Open a readable and executable memory map backed by a file
                ///
                /// # Examples
                ///
                #[doc = "```ignore"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFile, AsyncMmapFileExt};")]
                #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_exec_test.txt\").await.unwrap();")]
                #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_open_exec_test.txt\").unwrap());")]
                /// # file.truncate(12).await.unwrap();
                /// # file.write_all("some data...".as_bytes(), 0).unwrap();
                /// # file.flush().unwrap();
                /// # drop(file);
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncMmapFile::open_exec(\"", $filename_prefix, "_open_exec_test.txt\").await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                #[doc = "```"]
                pub async fn open_exec<P: AsRef<Path>>(path: P) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFile::open_exec(path).await?))
                }

                /// Open a readable and executable memory map backed by a file with [`Options`].
                ///
                /// # Examples
                ///
                #[doc = "```ignore"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFile, AsyncMmapFileExt};")]
                #[doc = concat!("# use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_exec_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!(" # defer!(std::fs::remove_file(\"", $filename_prefix, "_open_exec_with_options_test.txt\").unwrap());")]
                /// # file.truncate(23).await.unwrap();
                /// # file.write_all("sanity text".as_bytes(), 0).unwrap();
                /// # file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                /// # file.flush().unwrap();
                /// # drop(file);
                ///
                /// // mmap the file
                /// let opts = AsyncOptions::new()
                ///     // mmap content after the sanity text
                ///     .offset("sanity text".as_bytes().len() as u64);
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncMmapFile::open_exec_with_options(\"", $filename_prefix, "_open_exec_with_options_test.txt\", opts).await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
                pub async fn open_exec_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFile::open_exec_with_options(path, opts).await?))
                }
            }

            impl_constructor_for_memory_mmap_file!(AsyncMemoryMmapFile, AsyncMmapFile, "AsyncMmapFile", $path_str);
        };
    }

    macro_rules! delcare_and_impl_async_mmap_file_mut {
        ($filename_prefix: literal, $doc_test_runtime: literal, $path_str: literal) => {
            /// A writable memory map file.
            ///
            /// There is 3 status of this struct:
            /// - __Disk__: mmap to a real file
            /// - __Memory__: use [`BytesMut`] to mock a mmap, which is useful for test and in-memory storage engine
            /// - __Empty__: a state represents null mmap, which is helpful for drop, remove, close the `AsyncMmapFileMut`. This state cannot be constructed directly.
            ///
            /// [`BytesMut`]: https://docs.rs/bytes/1.1.0/bytes/struct.BytesMut.html
            pub struct AsyncMmapFileMut {
                inner: AsyncMmapFileMutInner,
                remove_on_drop: bool,
                deleted: bool,
            }

            impl_from_mut!(AsyncMmapFileMut, AsyncMmapFileMutInner, [AsyncEmptyMmapFile, AsyncMemoryMmapFileMut, AsyncDiskMmapFileMut]);

            impl_async_mmap_file_ext!(AsyncMmapFileMut);

            impl_async_mmap_file_mut_ext!($filename_prefix, $doc_test_runtime, $path_str);

            impl AsyncMmapFileMut {
                /// Create a new file and mmap this file
                ///
                /// # Notes
                /// The new file is zero size, so, before write, you should truncate first.
                /// Or you can use [`create_with_options`] and set `max_size` field for [`AsyncOptions`] to enable directly write
                /// without truncating.
                /// # Examples
                ///
                #[doc = "```ignore"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_create_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_create_test.txt\").unwrap());")]
                /// file.truncate(12).await;
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[`create_with_options`]: ", $path_str, "/struct.AsyncMmapFileMut.html#method.create_with_options")]
                #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
                pub async fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFileMut::create(path).await?))
                }

                /// Create a new file and mmap this file with [`AsyncOptions`]
                ///
                /// # Example
                ///
                #[doc = "```ignore"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncOptions, AsyncMmapFileMut, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                /// let opts = AsyncOptions::new()
                ///     // truncate to 100
                ///     .max_size(100);
                #[doc = concat!("let mut file = AsyncMmapFileMut::create_with_options(\"", $filename_prefix, "_create_with_options_test.txt\", opts).await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_create_with_options_test.txt\").unwrap());")]
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
                pub async fn create_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFileMut::create_with_options(path, opts).await?))
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
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_test.txt\").unwrap());")]
                /// # file.truncate(12).await.unwrap();
                /// # file.write_all("some data...".as_bytes(), 0).unwrap();
                /// # file.flush().unwrap();
                /// # drop(file);
                ///
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
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
                #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                /// File does not exists
                ///
                #[doc = "```ignore"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_test.txt\").unwrap());")]
                /// file.truncate(100).await.unwrap();
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
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
                #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_test.txt\").await.unwrap();")]
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[`open_with_options`]: ", $path_str, "/struct.AsyncMmapFileMut.html#method.open_with_options")]
                #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
                pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFileMut::open(path).await?))
                }

                /// Open or Create(if not exists) a file and mmap this file with [`AsyncOptions`].
                ///
                /// # Examples
                ///
                /// File already exists
                ///
                /// ```ignore
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("# let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_with_options_test.txt\").unwrap());")]
                /// # file.truncate(23).await.unwrap();
                /// # file.write_all("sanity text".as_bytes(), 0).unwrap();
                /// # file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                /// # file.flush().unwrap();
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
                #[doc = concat!("let mut file = AsyncMmapFileMut::open_with_options(\"", $filename_prefix, "_open_with_options_test.txt\", opts).await.unwrap();")]
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
                #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
                /// // skip the sanity text
                /// file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
                /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                /// File does not exists
                ///
                /// ```ignore
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
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
                #[doc = concat!("let mut file = AsyncMmapFileMut::open_with_options(\"", $filename_prefix, "_open_with_options_test.txt\", opts).await.unwrap();")]
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
                #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_with_options_test.txt\").await.unwrap();")]
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
                pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFileMut::open_with_options(path, opts).await?))
                }

                /// Open an existing file and mmap this file
                ///
                /// # Examples
                #[doc = "```ignore"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                /// // create a temp file
                #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_existing_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_existing_test.txt\").unwrap());")]
                /// file.truncate(12).await.unwrap();
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
                /// drop(file);
                ///
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncMmapFileMut::open_exist(\"", $filename_prefix, "_open_existing_test.txt\").await.unwrap();")]
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
                #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_existing_test.txt\").await.unwrap();")]
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
                pub async fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFileMut::open_exist(path).await?))
                }

                /// Open an existing file and mmap this file with [`AsyncOptions`]
                ///
                /// # Examples
                ///
                /// ```ignore
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                /// // create a temp file
                #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_existing_test_with_options.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_existing_test_with_options.txt\").unwrap());")]
                /// file.truncate(23).await.unwrap();
                /// file.write_all("sanity text".as_bytes(), 0).unwrap();
                /// file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                /// file.flush().unwrap();
                /// drop(file);
                ///
                /// // mmap the file
                /// let opts = AsyncOptions::new()
                ///     // truncate to 100
                ///     .max_size(100)
                ///     // mmap content after the sanity text
                ///     .offset("sanity text".as_bytes().len() as u64);
                ///
                #[doc = concat!("let mut file = AsyncMmapFileMut::open_exist_with_options(\"", $filename_prefix, "_open_existing_test_with_options.txt\", opts).await.unwrap();")]
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
                #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_existing_test_with_options.txt\").await.unwrap();")]
                /// let mut buf = vec![0; "some modified data...".len()];
                /// // skip the sanity text
                /// // file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
                /// // assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
                pub async fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFileMut::open_exist_with_options(path, opts).await?))
                }

                /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file).
                /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
                ///
                /// # Examples
                ///
                #[doc = "```ignore"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                /// // create a temp file
                #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_cow_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_cow_test.txt\").unwrap());")]
                /// file.truncate(12).await.unwrap();
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
                /// drop(file);
                ///
                /// // mmap the file
                #[doc = concat!("let mut file = AsyncMmapFileMut::open_cow(\"", $filename_prefix, "_open_cow_test.txt\").await.unwrap();")]
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
                #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_cow_test.txt\").await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
                pub async fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFileMut::open_cow(path).await?))
                }

                /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file) with [`AsyncOptions`].
                /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
                ///
                /// # Examples
                ///
                #[doc = "```ignore"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};")]
                /// use std::io::SeekFrom;
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                /// // create a temp file
                #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_open_cow_with_options_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_open_cow_with_options_test.txt\").unwrap());")]
                /// file.truncate(23).await.unwrap();
                /// file.write_all("sanity text".as_bytes(), 0).unwrap();
                /// file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                /// file.flush().unwrap();
                /// drop(file);
                ///
                /// // mmap the file
                /// let opts = AsyncOptions::new()
                ///     // mmap content after the sanity text
                ///     .offset("sanity text".as_bytes().len() as u64);
                ///
                #[doc = concat!("let mut file = AsyncMmapFileMut::open_cow_with_options(\"", $filename_prefix, "_open_cow_with_options_test.txt\", opts).await.unwrap();")]
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
                #[doc = concat!("let mut file = AsyncMmapFileMut::open(\"", $filename_prefix, "_open_cow_with_options_test.txt\").await.unwrap();")]
                /// let mut buf = vec![0; "some data...".len()];
                /// // skip the sanity text
                /// file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
                /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[`AsyncOptions`]: ", $path_str, "/struct.AsyncOptions.html")]
                pub async fn open_cow_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
                    Ok(Self::from(AsyncDiskMmapFileMut::open_cow_with_options(path, opts).await?))
                }

                /// Make the mmap file read-only.
                ///
                /// # Notes
                /// If `remove_on_drop` is set to `true`, then the underlying file will not be removed on drop if this function is invoked. [Read more]
                ///
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
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_freeze_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_freeze_test.txt\").unwrap());")]
                /// file.truncate(12).await;
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
                /// // freeze
                /// file.freeze().unwrap();
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[Read more]: ", $path_str, "/struct.AsyncMmapFileMut.html#methods.set_remove_on_drop")]
                ///
                #[inline]
                pub fn freeze(mut self) -> Result<AsyncMmapFile> {
                    let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
                    // swap the inner to empty
                    let inner = mem::replace(&mut self.inner, empty);
                    match inner {
                        AsyncMmapFileMutInner::Empty(empty) => Ok(AsyncMmapFile::from(empty)), // unreachable, keep this for good measure
                        AsyncMmapFileMutInner::Memory(memory) => Ok(AsyncMmapFile::from(memory.freeze())),
                        AsyncMmapFileMutInner::Disk(disk) => Ok(AsyncMmapFile::from(disk.freeze()?)),
                    }
                }

                /// Transition the memory map to be readable and executable.
                /// If the memory map is file-backed, the file must have been opened with execute permissions.
                ///
                /// # Notes
                /// If `remove_on_drop` is set to `true`, then the underlying file will not be removed on drop if this function is invoked. [Read more]
                ///
                /// # Errors
                /// This method returns an error when the underlying system call fails,
                /// which can happen for a variety of reasons,
                /// such as when the file has not been opened with execute permissions
                ///
                /// # Examples
                ///
                #[doc = "```ignore"]
                #[doc = concat!("use fmmap::", $path_str, "::{AsyncMmapFileMut, AsyncMmapFileMutExt};")]
                /// # use scopeguard::defer;
                ///
                #[doc = concat!("# ", $doc_test_runtime, "::block_on(async {")]
                #[doc = concat!("let mut file = AsyncMmapFileMut::create(\"", $filename_prefix, "_freeze_exec_test.txt\").await.unwrap();")]
                #[doc = concat!("# defer!(std::fs::remove_file(\"", $filename_prefix, "_freeze_exec_test.txt\").unwrap());")]
                /// file.truncate(12).await;
                /// file.write_all("some data...".as_bytes(), 0).unwrap();
                /// file.flush().unwrap();
                /// // freeze_exec
                /// file.freeze_exec().unwrap();
                /// # })
                #[doc = "```"]
                ///
                #[doc = concat!("[Read more]: ", $path_str, "/struct.AsyncMmapFileMut.html#methods.set_remove_on_drop")]
                ///
                #[inline]
                pub fn freeze_exec(mut self) -> Result<AsyncMmapFile> {
                    let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
                    // swap the inner to empty
                    let inner = mem::replace(&mut self.inner, empty);
                    match inner {
                        AsyncMmapFileMutInner::Empty(empty) => Ok(AsyncMmapFile::from(empty)), // unreachable, keep this for good measure
                        AsyncMmapFileMutInner::Memory(memory) => Ok(AsyncMmapFile::from(memory.freeze())),
                        AsyncMmapFileMutInner::Disk(disk) => Ok(AsyncMmapFile::from(disk.freeze_exec()?)),
                    }
                }

                /// Returns whether remove the underlying file on drop.
                #[inline]
                pub fn get_remove_on_drop(&self) -> bool {
                    self.remove_on_drop
                }

                /// Whether remove the underlying file on drop.
                /// Default is false.
                ///
                /// # Notes
                /// If invoke [`AsyncMmapFileMut::freeze`], then the file will
                /// not be removed even though the field `remove_on_drop` is true.
                ///
                /// [`AsyncMmapFileMut::freeze`]: structs.AsyncMmapFileMut.html#methods.freeze
                #[inline]
                pub fn set_remove_on_drop(&mut self, val: bool) {
                    self.remove_on_drop = val;
                }

                /// Close the file. It would also truncate the file if max_sz >= 0.
                #[inline]
                pub async fn close(&mut self, max_sz: i64) -> Result<()> {
                    let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
                    // swap the inner to empty
                    let inner = mem::replace(&mut self.inner, empty);
                    match inner {
                        AsyncMmapFileMutInner::Disk(disk) => {
                            disk.flush()?;
                            if max_sz >= 0 {
                                disk.file.set_len(max_sz as u64).await.map_err(From::from)
                            } else {
                                Ok(())
                            }
                        },
                        _ => Ok(()),
                    }
                }
            }

            impl_constructor_for_memory_mmap_file_mut!(AsyncMemoryMmapFileMut, AsyncMmapFileMut, "AsyncMmapFileMut", $path_str);

            impl_drop!(AsyncMmapFileMut, AsyncMmapFileMutInner, AsyncEmptyMmapFile);
        };
    }

    macro_rules! file_lock_tests {
        ($filename_prefix: literal, $runtime: meta) => {
            #[$runtime]
            async fn test_flush() {
                let path = concat!($filename_prefix, "_flush.txt");
                let mut file1 = AsyncMmapFileMut::create_with_options(path, AsyncOptions::new().max_size(100)).await.unwrap();
                file1.set_remove_on_drop(true);
                file1.write_all(vec![1; 100].as_slice(), 0).unwrap();
                file1.flush_range(0, 10).unwrap();
                file1.flush_async_range(11, 20).unwrap();
                file1.flush_async().unwrap();
            }

            #[$runtime]
            async fn test_lock_shared() {
                let path = concat!($filename_prefix, "_lock_shared.txt");
                let file1 = AsyncMmapFileMut::open(path).await.unwrap();
                let file2 = AsyncMmapFileMut::open(path).await.unwrap();
                let file3 = AsyncMmapFileMut::open(path).await.unwrap();
                defer!(std::fs::remove_file(path).unwrap());

                // Concurrent shared access is OK, but not shared and exclusive.
                file1.lock_shared().unwrap();
                file2.lock_shared().unwrap();
                assert!(file3.try_lock_exclusive().is_err());
                file1.unlock().unwrap();
                assert!(file3.try_lock_exclusive().is_err());

                // Once all shared file locks are dropped, an exclusive lock may be created;
                file2.unlock().unwrap();
                file3.lock_exclusive().unwrap();
            }

            #[$runtime]
            async fn test_lock_exclusive() {
                let path = concat!($filename_prefix, "_lock_exclusive.txt");
                defer!(std::fs::remove_file(path).unwrap());
                let file1 = AsyncMmapFileMut::open(path).await.unwrap();
                let file2 = AsyncMmapFileMut::open(path).await.unwrap();

                // No other access is possible once an exclusive lock is created.
                file1.lock_exclusive().unwrap();
                assert!(file2.try_lock_exclusive().is_err());
                assert!(file2.try_lock_shared().is_err());

                // Once the exclusive lock is dropped, the second file is able to create a lock.
                file1.unlock().unwrap();
                file2.lock_exclusive().unwrap();
            }

            #[$runtime]
            async fn test_lock_cleanup() {
                let path = concat!($filename_prefix, "_lock_cleanup.txt");
                defer!(std::fs::remove_file(path).unwrap());
                let file1 = AsyncMmapFileMut::open(path).await.unwrap();
                let file2 = AsyncMmapFileMut::open(path).await.unwrap();

                // No other access is possible once an exclusive lock is created.
                file1.lock_exclusive().unwrap();
                assert!(file2.try_lock_exclusive().is_err());
                assert!(file2.try_lock_shared().is_err());

                // Drop file1; the lock should be released.
                drop(file1);
                file2.lock_shared().unwrap();
            }
        };
    }
}

cfg_async_std!(
    pub(crate) mod async_std_impl;
);

cfg_smol!(
    pub(crate) mod smol_impl;
);

cfg_tokio!(
    pub(crate) mod tokio_impl;
);
