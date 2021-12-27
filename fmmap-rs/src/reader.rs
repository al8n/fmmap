cfg_sync!(
    mod sync_impl;
    pub use sync_impl::{MmapFileReader, MmapFileReaderExt};
);

cfg_tokio!(
    pub(crate) mod tokio_impl;
);
