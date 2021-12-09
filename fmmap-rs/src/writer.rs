cfg_sync!(
    mod sync_impl;
    pub use sync_impl::MmapFileWriter;
);
cfg_tokio!(
    pub(crate) mod tokio_impl;
);
