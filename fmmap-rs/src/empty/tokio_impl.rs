use std::path::{Path, PathBuf};
use crate::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncMmapFileReader, AsyncMmapFileWriter};
use crate::MetaData;
use crate::error::{Error, ErrorKind, Result};
use crate::metadata::EmptyMetaData;

declare_and_impl_async_empty_mmap_file!();

test_empty_mmap_file!(tokio::test);