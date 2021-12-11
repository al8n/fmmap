use tokio::fs::OpenOptions;
use memmap2::MmapOptions;

declare_and_impl_options!(AsyncOptions, OpenOptions);