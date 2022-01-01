// use std::path::{Path, PathBuf};
// #[cfg(not(target_os = "linux"))]
// use std::ptr::{drop_in_place, write};
// use async_trait::async_trait;
// use crate::MetaData;
// use crate::smol::{AsyncMmapFileExt, AsyncMmapFileMutExt};
// use crate::disk::{MmapFileMutType, remmap};
// use crate::error::Error;
// use crate::options::smol_impl::AsyncOptions;
// use crate::utils::smol::{create_file_async, open_exist_file_with_append_async, open_or_create_file_async, open_read_only_file_async, sync_dir_async};
// use fs4::tokio::AsyncFileExt;
// use memmap2::{Mmap, MmapMut, MmapOptions};
// use smol::fs::{File, remove_file};