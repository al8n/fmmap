use crate::{
  error::{Error, ErrorKind, Result},
  metadata::EmptyMetaData,
  tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncMmapFileReader, AsyncMmapFileWriter},
  MetaData,
};
use std::path::{Path, PathBuf};

declare_and_impl_async_empty_mmap_file!();

test_empty_mmap_file!(tokio::test);
