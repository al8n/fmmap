use std::path::Path;
use tokio::fs::OpenOptions;
use memmap2::MmapOptions;
use crate::tokio::{AsyncMmapFile, AsyncMmapFileMut};
use crate::error::Error;
use crate::raw::tokio::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};

declare_and_impl_async_options!("tokio_async", "tokio_test", "tokio");

impl_async_options_tests!("tokio_async", tokio::test, tokio);

#[cfg(unix)]
impl_options_unix_ext!(AsyncOptions);

#[cfg(windows)]
impl_options_windows_ext!(AsyncOptions);