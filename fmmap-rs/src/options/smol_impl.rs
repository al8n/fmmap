use std::path::Path;
use smol::fs::OpenOptions;
use smol::fs::unix::OpenOptionsExt;
use memmap2::MmapOptions;
use crate::smol::{AsyncMmapFile, AsyncMmapFileMut};
use crate::error::Error;
use crate::raw::smol::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};

declare_and_impl_async_options!("smol_async", "smol", "smol");