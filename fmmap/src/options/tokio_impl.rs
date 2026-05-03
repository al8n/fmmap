use crate::{
  error::Error,
  raw::tokio::{AsyncDiskMmapFile, AsyncDiskMmapFileMut},
  tokio::{AsyncMmapFile, AsyncMmapFileMut},
};
use memmapix::MmapOptions;
use std::path::Path;
use tokio::fs::OpenOptions;

declare_and_impl_async_options!("tokio_async", "tokio_test", "tokio");

impl_async_options_tests!("tokio_async", tokio::test, tokio);

#[cfg(unix)]
impl_options_unix_ext!(AsyncOptions);

#[cfg(windows)]
impl_options_windows_ext!(AsyncOptions);
