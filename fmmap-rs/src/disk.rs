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
                    self.file.metadata().map(MetaData::disk).map_err(Error::IO)
                }
            }
        };
    }

    pub(crate) mod sync_impl;
);

cfg_tokio!(
    pub(crate) mod tokio_impl;
);



