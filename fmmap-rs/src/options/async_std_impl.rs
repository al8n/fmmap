use async_std::path::Path;
use async_std::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

use memmap2::MmapOptions;
use crate::async_std::{AsyncMmapFile, AsyncMmapFileMut};
use crate::error::Error;
use crate::raw::async_std::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};

declare_and_impl_async_options!("async_std_async", "tokio_test", "async_std");

impl_async_options_tests!("std_async", async_std::test, async_std);