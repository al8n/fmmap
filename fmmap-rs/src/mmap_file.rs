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

macro_rules! impl_constructor_for_memory_mmap_file {
    ($memory_base: ident, $name: ident, $name_str: literal) => {
        use bytes::Bytes;

        impl $name {
            #[doc = concat!("Create a in-memory ", $name_str)]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = "use bytes::{BufMut, BytesMut};"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
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
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data = (0..=255u8).collect::<Vec<_>>();"]
            #[doc = concat!($name_str, "::memory_from_vec(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn memory_from_vec<P: AsRef<Path>>(path: P, src: Vec<u8>) -> Self {
                Self::from(<$memory_base>::from_vec(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from String")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::memory_from_string(\"foo.mem\", data.to_string());")]
            #[doc = "```"]
            pub fn memory_from_string<P: AsRef<Path>>(path: P, src: String) -> Self {
                Self::from(<$memory_base>::from_string(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from static slice")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = "use bytes::Bytes;"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static [u8] = \"some data...\".as_bytes();"]
            #[doc = concat!($name_str, "::memory_from_slice(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn memory_from_slice<P: AsRef<Path>>(path: P, src: &'static [u8]) -> Self {
                 Self::from(<$memory_base>::from_slice(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from static str")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = "use bytes::Bytes;"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::memory_from_str(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn memory_from_str<P: AsRef<Path>>(path: P, src: &'static str) -> Self {
                Self::from(<$memory_base>::from_str(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " by copy from slice")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
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
    ($memory_base: ident, $name: ident, $name_str: literal) => {
        impl $name {
            #[doc = concat!("Create a in-memory ", $name_str)]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!($name_str, "::memory(\"foo.mem\");")]
            #[doc = "```"]
            pub fn memory<P: AsRef<Path>>(path: P) -> Self {
                Self::from(<$memory_base>::new(path))
            }

            #[doc = concat!("Create a in-memory ", $name_str, "with capacity")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!($name_str, "::memory_with_capacity(\"foo.mem\", 1000);")]
            #[doc = "```"]
            pub fn memory_with_capacity<P: AsRef<Path>>(path: P, cap: usize) -> Self {
                Self::from(<$memory_base>::with_capacity(path, cap))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from Vec")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data = (0..=255u8).collect::<Vec<_>>();"]
            #[doc = concat!($name_str, "::memory_from_vec(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn memory_from_vec<P: AsRef<Path>>(path: P, src: Vec<u8>) -> Self {
                Self::from(<$memory_base>::from_vec(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from String")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::memory_from_string(\"foo.mem\", data.to_string());")]
            #[doc = "```"]
            pub fn memory_from_string<P: AsRef<Path>>(path: P, src: String) -> Self {
                Self::from(<$memory_base>::from_string(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " from static str")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = "use bytes::Bytes;"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::memory_from_str(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn memory_from_str<P: AsRef<Path>>(path: P, src: &'static str) -> Self {
                Self::from(<$memory_base>::from_str(path, src))
            }

            #[doc = concat!("Create a in-memory ", $name_str, " by from slice")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!($name_str, "::memory_from_slice(\"foo.mem\", \"some data...\".as_bytes());")]
            #[doc = "```"]
            pub fn memory_from_slice<P: AsRef<Path>>(path: P, src: &[u8]) -> Self {
                Self::from(<$memory_base>::from_slice(path, src))
            }
        }
    };
}

cfg_sync!(
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
            }
        };
    }

    mod sync_impl;
    pub use sync_impl::{MmapFileExt, MmapFileMutExt, MmapFile, MmapFileMut};
);

cfg_tokio!(
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
            }
        };
    }

    mod tokio_impl;
    pub use tokio_impl::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncMmapFile, AsyncMmapFileMut};
);
