use crate::error::Error;
use memmap2::{MmapAsRawDesc, MmapMut, MmapOptions};
use std::path::Path;

#[derive(Copy, Clone)]
enum MmapFileMutType {
    COW,
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
                MmapFileMutType::COW => MmapOptions::new().map_copy(file),
                MmapFileMutType::Normal => MmapMut::map_mut(file),
            },
            Some(opts) => {
                let opts = opts.clone();
                match typ {
                    MmapFileMutType::COW => opts.map_copy(file),
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

                fn metadata(&self) -> crate::error::Result<MetaData> {
                    self.file.metadata().map(MetaData::disk).map_err(Error::IO)
                }
            }
        };
    }

    mod sync_impl;
    pub use sync_impl::{DiskMmapFile, DiskMmapFileMut};
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

                async fn metadata(&self) -> crate::error::Result<MetaData> {
                    self.file
                        .metadata()
                        .await
                        .map(MetaData::disk)
                        .map_err(Error::IO)
                }
            }
        };
    }

    mod tokio_impl;
    pub use tokio_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};
);
