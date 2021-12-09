use std::fs::{File, remove_file};
use std::path::{Path, PathBuf};
use std::ptr::{drop_in_place, write};
use memmap2::{Mmap, MmapMut};
use crate::{MetaData, MmapFileExt, MmapFileMutExt};
use crate::error::Error;
use crate::metadata::DiskMetaData;

pub struct DiskMmapFile {
    mmap: Mmap,
    file: File,
    path: PathBuf,
}

impl_mmap_file_ext!(DiskMmapFile);

pub struct DiskMmapFileMut {
    mmap: MmapMut,
    file: File,
    path: PathBuf,
    remove_on_drop: bool,
}

impl_mmap_file_ext!(DiskMmapFileMut);

impl MmapFileMutExt for DiskMmapFileMut {
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.mmap.as_mut()
    }

    fn flush(&self) -> crate::error::Result<()> {
        self.mmap.flush().map_err(|e| Error::FlushFailed(format!("path: {:?}, err: {}", self.path(), e)))
    }

    fn flush_async(&self) -> crate::error::Result<()> {
        self.mmap.flush_async().map_err(|e| Error::FlushFailed(format!("path: {:?}, err: {}", self.path(), e)))
    }

    fn flush_range(&self, offset: usize, len: usize) -> crate::error::Result<()> {
        self.mmap.flush_range(offset, len).map_err(|e| Error::FlushFailed(format!("path: {:?}, err: {}", self.path(), e)))
    }

    fn flush_async_range(&self, offset: usize, len: usize) -> crate::error::Result<()> {
        self.mmap.flush_async_range(offset, len).map_err(|e| Error::FlushFailed(format!("path: {:?}, err: {}", self.path(), e)))
    }

    #[cfg(not(target_os = "linux"))]
    fn truncate(&mut self, max_sz: u64) -> crate::error::Result<()> {
        // sync data
        self.flush()?;

        unsafe {
            // unmap
            drop_in_place(&mut self.mmap);

            // truncate
            self.file.set_len(max_sz).map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", self.path(), e)))?;

            // remap
            let mmap = MmapMut::map_mut(&self.file).map_err(|e| Error::RemmapFailed(format!("path: {:?}, err: {}", self.path(), e)))?;

            write(&mut self.mmap, mmap);
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn truncate(&mut self, max_sz: u64) -> Result<()> {
        // sync data
        self.flush()?;

        // truncate
        self.file.set_len(max_sz).map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", self.path(), e)))?;

        // remap
        self.mmap = unsafe { MmapMut::map_mut(&self.file).map_err(|e| Error::RemmapFailed(format!("path: {:?}, err: {}", self.path(), e)))? };

        Ok(())
    }

    fn delete(self) -> crate::error::Result<()> {
        let path = self.path;
        drop(self.mmap);
        self.file.set_len(0).map_err(Error::IO)?;
        drop(self.file);
        remove_file(path).map_err(Error::IO)
    }

    fn close_with_truncate(self, max_sz: i64) -> crate::error::Result<()> {
        self.flush()?;
        drop(self.mmap);
        if max_sz >= 0 {
            self.file.set_len(max_sz as u64).map_err(Error::IO)?;
            let parent = self.path.parent().unwrap();
            File::open(parent).map_err(Error::IO)?.sync_all().map_err(|e| Error::SyncDirFailed(e.to_string()))?
        }
        Ok(())
    }
}

