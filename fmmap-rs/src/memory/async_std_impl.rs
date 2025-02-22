use async_std::path::{Path, PathBuf};
use std::time::SystemTime;

use bytes::{Bytes, BytesMut};
use crate::async_std::{AsyncMmapFileExt, AsyncMmapFileMutExt};
use crate::MetaData;
use crate::metadata::MemoryMetaData;

define_impl_constructor_for_mmap_file!(AsyncMemoryMmapFile, "AsyncMemoryMmapFile", "async_std::");

impl_async_mmap_file_ext!(AsyncMemoryMmapFile);

define_and_impl_constructor_for_mmap_file_mut!(AsyncMemoryMmapFileMut, "AsyncMemoryMmapFileMut", AsyncMemoryMmapFile, "AsyncMemoryMmapFile", "AsyncMmapFileExt", "async_std::");

impl_async_mmap_file_ext!(AsyncMemoryMmapFileMut);
impl_async_mmap_file_mut_ext!();