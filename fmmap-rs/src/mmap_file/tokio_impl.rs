use std::borrow::Cow;
use std::mem;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::fs::remove_file;
use crate::tokio::{AsyncMmapFileReader, AsyncMmapFileWriter, AsyncOptions};
use crate::disk::tokio_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};
use crate::empty::tokio_impl::AsyncEmptyMmapFile;
use crate::error::{Error, ErrorKind, Result};
use crate::memory::tokio_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut};
use crate::metadata::MetaData;

declare_async_mmap_file_ext!(AsyncDiskMmapFileMut, AsyncOptions, AsyncMmapFileReader);

declare_async_mmap_file_mut_ext!(AsyncMmapFileWriter);

declare_and_impl_inners!();

declare_and_impl_async_mmap_file!("tokio_async", "tokio_test", "tokio");

delcare_and_impl_async_mmap_file_mut!("tokio_async", "tokio_test", "tokio");

impl_async_tests!("tokio_async", tokio::test, tokio, AsyncMmapFile, AsyncMmapFileMut);