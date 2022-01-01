cfg_sync!(
    mod sync_impl;
    pub use sync_impl::{MmapFileWriter, MmapFileWriterExt};
);
cfg_tokio!(
    pub mod tokio_impl;
);
