macro_rules! define_impl_constructor_for_mmap_file {
    ($name: ident, $name_str: literal) => {
        /// Use [`Bytes`] to mock a mmap, which is useful for test and in-memory storage engine.
        ///
        /// [`Bytes`]: https://docs.rs/bytes/1.1.0/bytes/struct.Bytes.html
        pub struct $name {
            mmap: Bytes,
            path: PathBuf,
            create_at: SystemTime,
        }

        impl $name {
            #[doc = concat!("Create a ", $name_str)]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = "use bytes::{BufMut, BytesMut};"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let mut data = BytesMut::with_capacity(100);"]
            #[doc = "data.put_slice(\"some data...\".as_bytes());"]
            #[doc = concat!($name_str, "::new(\"foo.mem\", data.freeze());")]
            #[doc = "```"]
            pub fn new<P: AsRef<Path>>(path: P, data: Bytes) -> Self {
                Self {
                    mmap: data,
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now(),
                }
            }

            #[doc = concat!("Create a ", $name_str, " from Vec")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data = (0..=255u8).collect::<Vec<_>>();"]
            #[doc = concat!($name_str, "::from_vec(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn from_vec<P: AsRef<Path>>(path: P, src: Vec<u8>) -> Self {
                Self {
                    mmap: Bytes::from(src),
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now(),
                }
            }

            #[doc = concat!("Create a ", $name_str, " from String")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::from_string(\"foo.mem\", data.to_string());")]
            #[doc = "```"]
            pub fn from_string<P: AsRef<Path>>(path: P, src: String) -> Self {
                Self {
                    mmap: Bytes::from(src),
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now()
                }
            }

            #[doc = concat!("Create a ", $name_str, " from static slice")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = "use bytes::Bytes;"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static [u8] = \"some data...\".as_bytes();"]
            #[doc = concat!($name_str, "::from_slice(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn from_slice<P: AsRef<Path>>(path: P, src: &'static [u8]) -> Self {
                Self {
                    mmap: Bytes::from(src),
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now()
                }
            }

            #[doc = concat!("Create a ", $name_str, " from static str")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = "use bytes::Bytes;"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::from_str(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn from_str<P: AsRef<Path>>(path: P, src: &'static str) -> Self {
                Self {
                    mmap: Bytes::from(src),
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now()
                }
            }

            #[doc = concat!("Create a ", $name_str, " by copy from slice")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!($name_str, "::copy_from_slice(\"foo.mem\", \"some data...\".as_bytes());")]
            #[doc = "```"]
            pub fn copy_from_slice<P: AsRef<Path>>(path: P, src: &[u8]) -> Self {
                Self {
                    mmap: Bytes::copy_from_slice(src),
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now(),
                }
            }

            #[doc = "Returns the inner bytes"]
            #[doc = "# Examples"]
            #[doc =  "```rust"]
            #[doc = "use bytes::Bytes;"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!("let b1 = ", $name_str, "::copy_from_slice(\"foo.mem\", \"some data...\".as_bytes()).into_bytes();")]
            #[doc = "assert_eq!(b1, Bytes::copy_from_slice(\"some data...\".as_bytes()));"]
            #[doc = "```"]
            pub fn into_bytes(self) -> Bytes {
                self.mmap
            }
        }
    };
}

macro_rules! define_and_impl_constructor_for_mmap_file_mut {
    ($name: ident, $name_str: literal) => {
        #[doc = "Use [`BytesMut`] to mock a mmap, which is useful for test and in-memory storage engine."]
        #[doc = ""]
        #[doc = "# Notes"]
        #[doc = concat!($name_str, " mocks a mmap behaviour, which means when writing to it,")]
        #[doc = "it will not auto-grow its size, so if you want to grow the size of the MemoryMmapFileMut,"]
        #[doc = "you need to [`truncate`] it first."]
        #[doc = ""]
        #[doc = "If you want the auto-grow functionality, please use [`BytesMut`]."]
        #[doc = ""]
        #[doc = "[`truncate`]: structs.MemoryMmapFileMut.html#methods.truncate"]
        #[doc = "[`BytesMut`]: https://docs.rs/bytes/1.1.0/bytes/struct.BytesMut.html"]
        pub struct $name {
            mmap: BytesMut,
            path: PathBuf,
            create_at: SystemTime,
        }

        impl $name {
            #[doc = concat!("Create a ", $name_str)]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!($name_str, "::new(\"foo.mem\");")]
            #[doc = "```"]
            pub fn new<P: AsRef<Path>>(path: P) -> Self {
                Self {
                    mmap: BytesMut::new(),
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now(),
                }
            }

            #[doc = concat!("Create a ", $name_str, "with capacity")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!($name_str, "::with_capacity(\"foo.mem\", 1000);")]
            #[doc = "```"]
            pub fn with_capacity<P: AsRef<Path>>(path: P, cap: usize) -> Self {
                Self {
                    mmap: BytesMut::with_capacity(cap),
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now(),
                }
            }

            #[doc = concat!("Create a ", $name_str, " from Vec")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data = (0..=255u8).collect::<Vec<_>>();"]
            #[doc = concat!($name_str, "::from_vec(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn from_vec<P: AsRef<Path>>(path: P, src: Vec<u8>) -> Self {
                Self {
                    mmap: BytesMut::from_iter(src),
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now(),
                }
            }

            #[doc = concat!("Create a ", $name_str, " from String")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::from_string(\"foo.mem\", data.to_string());")]
            #[doc = "```"]
            pub fn from_string<P: AsRef<Path>>(path: P, src: String) -> Self {
                Self {
                    mmap: BytesMut::from(src.as_bytes()),
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now()
                }
            }

            #[doc = concat!("Create a ", $name_str, " from static str")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = "use bytes::Bytes;"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = "let data: &'static str = \"some data...\";"]
            #[doc = concat!($name_str, "::from_str(\"foo.mem\", data);")]
            #[doc = "```"]
            pub fn from_str<P: AsRef<Path>>(path: P, src: &'static str) -> Self {
                Self {
                    mmap: BytesMut::from(src),
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now()
                }
            }

            #[doc = concat!("Create a ", $name_str, " by from slice")]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!($name_str, "::from_slice(\"foo.mem\", \"some data...\".as_bytes());")]
            #[doc = "```"]
            pub fn from_slice<P: AsRef<Path>>(path: P, src: &[u8]) -> Self {
                Self {
                    mmap: BytesMut::from(src),
                    path: path.as_ref().to_path_buf(),
                    create_at: SystemTime::now()
                }
            }

            #[doc = "Returns the inner mutable bytes"]
            #[doc = "# Examples"]
            #[doc =  "```rust"]
            #[doc = "use bytes::BytesMut;"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!("let b1 = ", $name_str, "::from_slice(\"foo.mem\", \"some data...\".as_bytes()).into_bytes();")]
            #[doc = "assert_eq!(b1, BytesMut::from(\"some data...\".as_bytes()));"]
            #[doc = "```"]
            pub fn into_bytes_mut(self) -> BytesMut {
                self.mmap
            }

            #[doc = "Returns the inner bytes"]
            #[doc = "# Examples"]
            #[doc = "```rust"]
            #[doc = "use bytes::Bytes;"]
            #[doc = concat!("use fmmap::raw::", $name_str, ";")]
            #[doc = ""]
            #[doc = concat!("let b1 = ", $name_str, "::from_slice(\"foo.mem\", \"some data...\".as_bytes()).into_bytes();")]
            #[doc = "assert_eq!(b1, Bytes::copy_from_slice(\"some data...\".as_bytes()));"]
            #[doc = "```"]
            pub fn into_bytes(self) -> Bytes {
                self.mmap.freeze()
            }
        }
    };
}

cfg_sync!(
    macro_rules! impl_mmap_file_ext {
        ($name: ident) => {
            impl MmapFileExt for $name {
                fn len(&self) -> usize {
                    self.mmap.len()
                }

                fn as_slice(&self) -> &[u8] {
                    self.mmap.as_ref()
                }

                fn path(&self) -> &Path {
                    self.path.as_path()
                }

                fn is_exec(&self) -> bool {
                    false
                }

                fn metadata(&self) -> crate::error::Result<MetaData> {
                    Ok(MetaData::memory(MemoryMetaData::new(
                        self.mmap.len() as u64,
                        self.create_at,
                    )))
                }
            }
        };
    }
    mod sync_impl;
    pub use sync_impl::{MemoryMmapFile, MemoryMmapFileMut};
);

cfg_tokio!(
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

                fn is_exec(&self) -> bool {
                    false
                }

                async fn metadata(&self) -> crate::error::Result<MetaData> {
                    Ok(MetaData::memory(MemoryMetaData::new(
                        self.mmap.len() as u64,
                        self.create_at,
                    )))
                }
            }
        };
    }

    mod tokio_impl;
    pub use tokio_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut};
);
