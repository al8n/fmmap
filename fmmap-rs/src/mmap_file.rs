macro_rules! read_impl {
    ($this:ident, $offset: tt, $typ:tt::$conv:tt) => {{
        const SIZE: usize = mem::size_of::<$typ>();
        // try to convert directly from the bytes
        // this Option<ret> trick is to avoid keeping a borrow on self
        // when advance() is called (mut borrow) and to call bytes() only once
        let mut buf = [0; SIZE];
        $this
            .read_exact(&mut buf, $offset)
            .map(|src| unsafe { $typ::$conv(*(&src as *const _ as *const [_; SIZE])) })
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

cfg_sync!(
    macro_rules! impl_mmap_file_ext {
        ($name: ident) => {
            impl MmapFileExt for $name {
                fn len(&self) -> usize {
                    self.inner.len()
                }

                fn as_slice(&self) -> &[u8] {
                    self.inner.as_slice()
                }

                fn path(&self) -> &Path {
                    self.inner.path()
                }

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
                fn len(&self) -> usize {
                    self.inner.len()
                }

                fn as_slice(&self) -> &[u8] {
                    self.inner.as_slice()
                }

                fn path(&self) -> &Path {
                    self.inner.path()
                }

                async fn metadata(&self) -> Result<MetaData> {
                    self.inner.metadata().await
                }
            }
        };
    }

    mod tokio_impl;
    pub use tokio_impl::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncMmapFile, AsyncMmapFileMut};
);
