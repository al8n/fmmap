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
