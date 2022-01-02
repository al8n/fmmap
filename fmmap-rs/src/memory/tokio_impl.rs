use std::path::{Path, PathBuf};
use std::time::SystemTime;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use crate::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt};
use crate::MetaData;
use crate::metadata::MemoryMetaData;

define_impl_constructor_for_mmap_file!(AsyncMemoryMmapFile, "AsyncMemoryMmapFile", "tokio::");

impl_async_mmap_file_ext!(AsyncMemoryMmapFile);

define_and_impl_constructor_for_mmap_file_mut!(AsyncMemoryMmapFileMut, "AsyncMemoryMmapFileMut", AsyncMemoryMmapFile, "AsyncMemoryMmapFile", "AsyncMmapFileExt", "tokio::");

impl_async_mmap_file_ext!(AsyncMemoryMmapFileMut);
impl_async_mmap_file_mut_ext!();