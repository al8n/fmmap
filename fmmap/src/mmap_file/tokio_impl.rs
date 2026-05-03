use crate::{
  disk::tokio_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut},
  empty::tokio_impl::AsyncEmptyMmapFile,
  error::{Error, ErrorKind, Result},
  memory::tokio_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut},
  metadata::MetaData,
  tokio::{AsyncMmapFileReader, AsyncMmapFileWriter, AsyncOptions},
  utils::tokio::sync_dir_async,
};
use std::{
  borrow::Cow,
  io::Cursor,
  mem,
  path::{Path, PathBuf},
};
use tokio::{fs::remove_file, io::AsyncWriteExt};

declare_async_mmap_file_ext!(
  AsyncDiskMmapFileMut,
  AsyncOptions,
  AsyncMmapFileReader<'_>,
  ::tokio::fs::OpenOptions
);

declare_async_mmap_file_mut_ext!(AsyncMmapFileWriter<'_>);

declare_and_impl_inners!();

declare_and_impl_async_mmap_file!("tokio_async", "tokio_test", "tokio");

delcare_and_impl_async_mmap_file_mut!("tokio_async", "tokio_test", "tokio");

impl_async_tests!(
  "tokio_async",
  tokio::test,
  tokio,
  AsyncMmapFile,
  AsyncMmapFileMut
);
