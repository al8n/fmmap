use async_std::path::{Path, PathBuf};
use crate::async_std::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncMmapFileReader, AsyncMmapFileWriter};
use crate::MetaData;
use crate::error::{Error, Result};
use crate::metadata::EmptyMetaData;

declare_and_impl_async_empty_mmap_file!();

test_empty_mmap_file!(async_std::test);