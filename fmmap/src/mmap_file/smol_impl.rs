use std::{
  borrow::Cow,
  mem,
  path::{Path, PathBuf},
};

use crate::{
  disk::smol_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut},
  empty::smol_impl::AsyncEmptyMmapFile,
  error::{Error, ErrorKind, Result},
  memory::smol_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut},
  metadata::MetaData,
  smol::{AsyncMmapFileReader, AsyncMmapFileWriter, AsyncOptions},
  utils::smol::sync_dir_async,
};
use smol::{
  fs::remove_file,
  io::{AsyncWriteExt, Cursor},
};

declare_async_mmap_file_ext!(
  AsyncDiskMmapFileMut,
  AsyncOptions,
  AsyncMmapFileReader<'_>,
  ::smol::fs::OpenOptions
);

declare_async_mmap_file_mut_ext!(AsyncMmapFileWriter<'_>);

declare_and_impl_inners!();

declare_and_impl_async_mmap_file!("smol_async", "smol", "smol");

delcare_and_impl_async_mmap_file_mut!("smol_async", "smol", "smol");

impl_async_tests!(
  "smol_async",
  smol_potat::test,
  smol,
  AsyncMmapFile,
  AsyncMmapFileMut
);
