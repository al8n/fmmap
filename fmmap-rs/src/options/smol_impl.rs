use crate::error::Error;
use crate::raw::smol::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};
use crate::smol::{AsyncMmapFile, AsyncMmapFileMut};
use memmap2::MmapOptions;
#[cfg(unix)]
use smol::fs::unix::OpenOptionsExt;
#[cfg(windows)]
use smol::fs::windows::OpenOptionsExt;
use smol::fs::OpenOptions;
use std::path::Path;

declare_and_impl_async_options!("smol_async", "smol", "smol");

impl_async_options_tests!("smol_async", smol_potat::test, smol);

#[cfg(unix)]
impl_options_unix_ext!(AsyncOptions);

#[cfg(windows)]
impl_options_windows_ext!(AsyncOptions);
