use std::{
  path::{Path, PathBuf},
  time::SystemTime,
};

use crate::{
  metadata::MemoryMetaData,
  tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt},
  MetaData,
};
use bytes::{Bytes, BytesMut};

define_impl_constructor_for_mmap_file!(AsyncMemoryMmapFile, "AsyncMemoryMmapFile", "tokio::");

impl_async_mmap_file_ext!(AsyncMemoryMmapFile);

define_and_impl_constructor_for_mmap_file_mut!(
  AsyncMemoryMmapFileMut,
  "AsyncMemoryMmapFileMut",
  AsyncMemoryMmapFile,
  "AsyncMemoryMmapFile",
  "AsyncMmapFileExt",
  "tokio::"
);

impl_async_mmap_file_ext!(AsyncMemoryMmapFileMut);
impl_async_mmap_file_mut_ext!();
