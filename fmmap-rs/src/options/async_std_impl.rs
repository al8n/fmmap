use async_std::fs::OpenOptions;
#[cfg(unix)]
use async_std::os::unix::fs::OpenOptionsExt;
use async_std::path::Path;

use crate::async_std::{AsyncMmapFile, AsyncMmapFileMut};
use crate::error::Error;
use crate::raw::async_std::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};
use memmapix::MmapOptions;

declare_and_impl_async_options!("async_std_async", "tokio_test", "async_std");

impl_async_options_tests!("std_async", async_std::test, async_std);

#[cfg(unix)]
impl_options_unix_ext!(AsyncOptions);
