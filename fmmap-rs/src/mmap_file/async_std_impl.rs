use std::borrow::Cow;
use std::mem;
use async_std::path::{Path, PathBuf};
use async_trait::async_trait;
use async_std::io::{WriteExt as AsyncWriteExt, Cursor};
use crate::async_std::{AsyncMmapFileReader, AsyncMmapFileWriter, AsyncOptions};
use crate::disk::async_std_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};
use crate::empty::async_std_impl::AsyncEmptyMmapFile;
use crate::error::{Error, Result};
use crate::memory::async_std_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut};
use crate::metadata::MetaData;

declare_async_mmap_file_ext!(AsyncDiskMmapFileMut, AsyncOptions, AsyncMmapFileReader);

declare_async_mmap_file_mut_ext!(AsyncMmapFileWriter);

#[enum_dispatch(AsyncMmapFileExt)]
enum AsyncMmapFileInner {
    Empty(AsyncEmptyMmapFile),
    Memory(AsyncMemoryMmapFile),
    Disk(AsyncDiskMmapFile)
}

declare_and_impl_async_mmap_file!("async_std_async", "async_std::task", "async_std");

#[enum_dispatch(AsyncMmapFileExt, AsyncMmapFileMutExt)]
enum AsyncMmapFileMutInner {
    Empty(AsyncEmptyMmapFile),
    Memory(AsyncMemoryMmapFileMut),
    Disk(AsyncDiskMmapFileMut)
}

delcare_and_impl_async_mmap_file_mut!("async_std_async", "async_std::task", "async_std");