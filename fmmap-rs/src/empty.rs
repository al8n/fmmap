cfg_sync!(
    mod sync_impl;
    pub use sync_impl::EmptyMmapFile;
);

cfg_tokio!(
    mod tokio_impl;
    pub use tokio_impl::AsyncEmptyMmapFile;
);
