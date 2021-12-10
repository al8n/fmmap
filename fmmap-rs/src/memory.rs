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

                fn stat(&self) -> crate::error::Result<MetaData> {
                    Ok(MetaData::memory(MemoryMetaData::new(self.mmap.len() as u64, self.create_at)))
                }
            }
        };
    }
    pub(crate) mod sync_impl;
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

                async fn stat(&self) -> crate::error::Result<MetaData> {
                    Ok(MetaData::memory(MemoryMetaData::new(self.mmap.len() as u64, self.create_at)))
                }
            }
        };
    }

    pub(crate) mod tokio_impl;
);


