use std::path::Path;
use smol::fs::OpenOptions;
#[cfg(unix)]
use smol::fs::unix::OpenOptionsExt;
#[cfg(windows)]
use smol::fs::windows::OpenOptionsExt;
use memmap2::MmapOptions;
use crate::smol::{AsyncMmapFile, AsyncMmapFileMut};
use crate::error::Error;
use crate::raw::smol::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};

declare_and_impl_async_options!("smol_async", "smol", "smol");

impl_async_options_tests!("smol_async", smol_potat::test, smol);