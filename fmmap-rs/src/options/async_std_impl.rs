use async_std::path::Path;
use async_std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use memmap2::MmapOptions;
use crate::async_std::{AsyncMmapFile, AsyncMmapFileMut};
use crate::error::Error;
use crate::raw::async_std::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};

declare_and_impl_async_options!("async_std_async", "tokio_test", "async_std");