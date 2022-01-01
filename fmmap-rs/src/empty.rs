cfg_sync!(
    mod sync_impl;
    pub use sync_impl::EmptyMmapFile;
);

cfg_tokio!(
    pub(crate) mod tokio_impl;
);
