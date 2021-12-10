use tokio::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
use memmap2::MmapOptions;

declare_and_impl_options!(AsyncOptions, OpenOptions);