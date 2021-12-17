use std::borrow::Cow;
use std::mem;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use crate::error::{Error, Result};
use crate::metadata::MetaData;
use crate::{MmapFileReader, MmapFileWriter};
use crate::disk::{DiskMmapFile, DiskMmapFileMut};
use crate::empty::EmptyMmapFile;
use crate::memory::{MemoryMmapFile, MemoryMmapFileMut};
use crate::options::Options;

/// Utility methods to [`MmapFile`]
///
/// [`MmapFile`]: structs.MmapFile.html
#[enum_dispatch]
pub trait MmapFileExt {
    /// Returns the current mmap length
    fn len(&self) -> usize;

    /// Returns the mmap is empty of not.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the underlying slice of the mmap
    fn as_slice(&self) -> &[u8];

    /// slice returns data starting from offset off of size sz.
    ///
    /// # Panics
    /// If there's not enough data, it would
    /// panic.
    fn slice(&self, offset: usize, sz: usize) -> &[u8] {
        &self.as_slice()[offset..offset+sz]
    }

    /// bytes returns data starting from offset off of size sz.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    /// `Err(Error::EOF)`.
    fn bytes(&self, offset: usize, sz: usize) -> Result<&[u8]> {
        let buf = self.as_slice();
        if buf.len() < offset + sz {
            Err(Error::EOF)
        } else {
            Ok(&buf[offset..offset+sz])
        }
    }

    /// Returns the path of the inner file.
    fn path(&self) -> &Path;

    /// Returns the path buf of the inner file.
    fn path_buf(&self) -> PathBuf {
        self.path().to_path_buf()
    }

    /// Returns the path lossy string of the inner file.
    fn path_lossy(&self) -> Cow<'_, str> {
        self.path().to_string_lossy()
    }

    /// Returns the path string of the inner file.
    fn path_string(&self) -> String {
        self.path_lossy().to_string()
    }

    /// Returns the metadata of file metadata
    ///
    /// Metadata information about a file.
    /// This structure is returned from the metadata or
    /// symlink_metadata function or method and represents
    /// known metadata about a file such as its permissions, size, modification times, etc
    fn metadata(&self) -> Result<MetaData>;

    /// Whether the mmap is executable.
    fn is_exec(&self) -> bool;

    /// Copy the content of the mmap file to Vec
    #[inline]
    fn copy_all_to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }

    /// Copy a range of content of the mmap file to Vec
    #[inline]
    fn copy_range_to_vec(&self, offset: usize, len: usize) -> Vec<u8> {
        self.slice(offset, len).to_vec()
    }

    /// Write the content of the mmap file to a new file.
    #[inline]
    fn write_all_to_new_file<P: AsRef<Path>>(&self, new_file_path: P) -> Result<()> {
        let buf = self.as_slice();
        let mut opts = Options::new();
        opts.max_size(buf.len() as u64);

        let mut mmap = DiskMmapFileMut::create_with_options(new_file_path, opts)?;
        mmap.writer(0)?.write_all(buf)?;
        mmap.flush()
    }

    /// Write a range of content of the mmap file to new file.
    #[inline]
    fn write_range_to_new_file<P: AsRef<Path>>(&self, new_file_path: P, offset: usize, len: usize) -> Result<()> {
        let buf = self.as_slice();
        if buf.len() < offset + len {
            return Err(Error::EOF)
        }
        let mut opts = Options::new();
        opts.max_size(len as u64);

        let mut mmap = DiskMmapFileMut::create_with_options(new_file_path, opts)?;
        mmap.writer(0)?.write_all(&buf[offset..offset + len])?;
        mmap.flush()
    }

    /// Returns a [`MmapFileReader`] which helps read data from mmap like a normal File.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    ///  `Err(Error::EOF)`.
    ///
    /// [`MmapFileReader`]: structs.MmapFileReader.html
    fn reader(&self, offset: usize) -> Result<MmapFileReader> {
        let buf = self.as_slice();
        if buf.len() < offset {
            Err(Error::EOF)
        } else {
            Ok(MmapFileReader::new(Cursor::new(&buf[offset..]), offset, buf.len() - offset))
        }
    }

    /// Returns a [`MmapFileReader`] base on the given `offset` and `len`, which helps read data from mmap like a normal File.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    ///  `Err(Error::EOF)`.
    ///
    /// [`MmapFileReader`]: structs.MmapFileReader.html
    fn range_reader(&self, offset: usize, len: usize) -> Result<MmapFileReader> {
        let buf = self.as_slice();
        if buf.len() < offset + len {
            Err(Error::EOF)
        } else {
            Ok(MmapFileReader::new(Cursor::new(&buf[offset.. offset + len]), offset, len))
        }
    }

    impl_read_ext!();
}

/// Utility methods to [`MmapFileMut`]
///
/// [`MmapFileMut`]: structs.MmapFileMut.html
#[enum_dispatch]
pub trait MmapFileMutExt {
    /// Returns the mutable underlying slice of the mmap
    fn as_mut_slice(&mut self) -> &mut [u8];

    /// slice_mut returns mutable data starting from offset off of size sz.
    ///
    /// # Panics
    /// If there's not enough data, it would
    /// panic.
    fn slice_mut(&mut self, offset: usize, sz: usize) -> &mut [u8] {
        &mut self.as_mut_slice()[offset..offset+sz]
    }

    /// Whether mmap is copy on write
    fn is_cow(&self) -> bool;

    /// bytes_mut returns mutable data starting from offset off of size sz.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    /// `Err(Error::EOF)`.
    fn bytes_mut(&mut self, offset: usize, sz: usize) -> Result<&mut [u8]> {
        let buf = self.as_mut_slice();
        if buf.len() <= offset + sz {
            Err(Error::EOF)
        } else {
            Ok(&mut buf[offset..offset+sz])
        }
    }

    /// Fill 0 to the specific range
    fn zero_range(&mut self, start: usize, end: usize) {
        let buf = self.as_mut_slice();
        let end = end.min(buf.len());
        buf[start..end].fill(0);
    }

    /// Flushes outstanding memory map modifications to disk (if the inner is a real file).
    ///
    /// When this method returns with a non-error result,
    /// all outstanding changes to a file-backed memory map are guaranteed to be durably stored.
    /// The file’s metadata (including last modification timestamp) may not be updated.
    fn flush(&self) -> Result<()>;

    /// Asynchronously flushes outstanding memory map modifications to disk(if the inner is a real file).
    ///
    /// This method initiates flushing modified pages to durable storage,
    /// but it will not wait for the operation to complete before returning.
    /// The file’s metadata (including last modification timestamp) may not be updated.
    fn flush_async(&self) -> Result<()>;

    /// Flushes outstanding memory map modifications in the range to disk(if the inner is a real file).
    ///
    /// The offset and length must be in the bounds of the memory map.
    ///
    /// When this method returns with a non-error result,
    /// all outstanding changes to a file-backed memory
    /// in the range are guaranteed to be durable stored.
    /// The file’s metadata (including last modification timestamp) may not be updated.
    /// It is not guaranteed the only the changes in the specified range are flushed;
    /// other outstanding changes to the memory map may be flushed as well.
    fn flush_range(&self, offset: usize, len: usize) -> Result<()>;

    /// Asynchronously flushes outstanding memory map modifications in the range to disk(if the inner is a real file).
    ///
    /// The offset and length must be in the bounds of the memory map.
    ///
    /// This method initiates flushing modified pages to durable storage,
    /// but it will not wait for the operation to complete before returning.
    /// The file’s metadata (including last modification timestamp) may not be updated.
    /// It is not guaranteed that the only changes flushed are those in the specified range;
    /// other outstanding changes to the memory map may be flushed as well.
    fn flush_async_range(&self, offset: usize, len: usize) -> Result<()>;

    /// Truncates the file to the `max_size`, which will lead to
    /// do re-mmap and sync_dir if the inner is a real file.
    fn truncate(&mut self, max_sz: u64) -> Result<()>;

    /// Remove the underlying file
    fn remove(self) -> Result<()>;

    /// Close and truncate the underlying file
    fn close_with_truncate(self, max_sz: i64) -> Result<()>;

    /// Returns a [`MmapFileWriter`] base on the given `offset`, which helps read or write data from mmap like a normal File.
    ///
    /// # Notes
    /// If you use a writer to write data to mmap, there is no guarantee all
    /// data will be durably stored. So you need to call [`flush`]/[`flush_range`]/[`flush_async`]/[`flush_async_range`] in [`MmapFileMutExt`]
    /// to guarantee all data will be durably stored.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    ///  `Err(Error::EOF)`.
    ///
    /// [`flush`]: traits.MmapFileMutExt.html#methods.flush
    /// [`flush_range`]: traits.MmapFileMutExt.html#methods.flush_range
    /// [`flush_async`]: traits.MmapFileMutExt.html#methods.flush_async
    /// [`flush_async_range`]: traits.MmapFileMutExt.html#methods.flush_async_range
    /// [`MmapFileWriter`]: structs.MmapFileWriter.html
    fn writer(&mut self, offset: usize) -> Result<MmapFileWriter> {
        let buf = self.as_mut_slice();
        let buf_len = buf.len();
        if buf_len <= offset {
            Err(Error::EOF)
        } else {
            Ok(MmapFileWriter::new(Cursor::new(&mut buf[offset..]), offset, buf_len - offset))
        }
    }

    /// Returns a [`MmapFileWriter`] base on the given `offset` and `len`, which helps read or write data from mmap like a normal File.
    ///
    /// # Notes
    /// If you use a writer to write data to mmap, there is no guarantee all
    /// data will be durably stored. So you need to call [`flush`]/[`flush_range`]/[`flush_async`]/[`flush_async_range`] in [`MmapFileMutExt`]
    /// to guarantee all data will be durably stored.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    ///  `Err(Error::EOF)`.
    ///
    /// [`flush`]: traits.MmapFileMutExt.html#methods.flush
    /// [`flush_range`]: traits.MmapFileMutExt.html#methods.flush_range
    /// [`flush_async`]: traits.MmapFileMutExt.html#methods.flush_async
    /// [`flush_async_range`]: traits.MmapFileMutExt.html#methods.flush_async_range
    /// [`MmapFileWriter`]: structs.MmapFileWriter.html
    fn range_writer(&mut self, offset: usize, len: usize) -> Result<MmapFileWriter> {
        let buf = self.as_mut_slice();
        if buf.len() < offset + len {
            Err(Error::EOF)
        } else {
            Ok(MmapFileWriter::new(
                Cursor::new(&mut buf[offset..offset + len]), offset, len))
        }
    }

    impl_write_ext!();
}

#[enum_dispatch(MmapFileExt)]
enum MmapFileInner {
    Empty(EmptyMmapFile),
    Memory(MemoryMmapFile),
    Disk(DiskMmapFile)
}

/// A read-only memory map file.
///
/// There is 3 status of this struct:
/// - __Disk__: mmap to a real file
/// - __Memory__: use [`Bytes`] to mock a mmap, which is useful for test and in-memory storage engine
/// - __Empty__: a state represents null mmap, which is helpful for drop, close the `MmapFile`. This state cannot be constructed directly.
///
/// [`Bytes`]: https://docs.rs/bytes/1.1.0/bytes/struct.Bytes.html
#[repr(transparent)]
pub struct MmapFile {
    inner: MmapFileInner,
}

impl_mmap_file_ext!(MmapFile);

impl_from!(MmapFile, MmapFileInner, [EmptyMmapFile, MemoryMmapFile, DiskMmapFile]);

#[enum_dispatch(MmapFileExt, MmapFileMutExt)]
enum MmapFileMutInner {
    Empty(EmptyMmapFile),
    Memory(MemoryMmapFileMut),
    Disk(DiskMmapFileMut)
}

/// A writable memory map file.
///
/// There is 3 status of this struct:
/// - __Disk__: mmap to a real file
/// - __Memory__: use [`BytesMut`] to mock a mmap, which is useful for test and in-memory storage engine
/// - __Empty__: a state represents null mmap, which is helpful for drop, remove, close the `MmapFileMut`. This state cannot be constructed directly.
///
/// [`BytesMut`]: https://docs.rs/bytes/1.1.0/bytes/struct.BytesMut.html
pub struct MmapFileMut {
    inner: MmapFileMutInner,
    remove_on_drop: bool,
    deleted: bool,
}

impl_from_mut!(MmapFileMut, MmapFileMutInner, [EmptyMmapFile, MemoryMmapFileMut, DiskMmapFileMut]);

impl_mmap_file_ext!(MmapFileMut);

impl MmapFileMutExt for MmapFileMut {
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.inner.as_mut_slice()
    }

    fn is_cow(&self) -> bool {
        self.inner.is_cow()
    }

    impl_flush!();

    fn truncate(&mut self, max_sz: u64) -> Result<()> {
        self.inner.truncate(max_sz)
    }

    fn remove(mut self) -> Result<()> {
        let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        if self.remove_on_drop {
            // do remove
            inner.remove()?;
            self.deleted = true;
        }
        Ok(())
    }

    fn close_with_truncate(mut self, max_sz: i64) -> Result<()> {
        let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        inner.close_with_truncate(max_sz)
    }
}

impl MmapFileMut {
    /// Make the mmap file read-only.
    ///
    /// # Notes
    /// If `remove_on_drop` is set to `true`, then the underlying file will not be removed on drop if this function is invoked. [Read more]
    ///
    /// [Read more]: structs.MmapFileMut.html#methods.set_remove_on_drop
    pub fn freeze(mut self) -> Result<MmapFile> {
        let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        match inner {
            MmapFileMutInner::Empty(empty) => Ok(MmapFile::from(empty)),
            MmapFileMutInner::Memory(memory) => Ok(MmapFile::from(memory.freeze())),
            MmapFileMutInner::Disk(disk) => Ok(MmapFile::from(disk.freeze()?)),
        }
    }

    /// Returns whether remove the underlying file on drop.
    #[inline]
    pub fn get_remove_on_drop(&self) -> bool {
        self.remove_on_drop
    }

    /// Whether remove the underlying file on drop.
    /// Default is false.
    ///
    /// # Notes
    /// If invoke [`MmapFileMut::freeze`], then the file will
    /// not be removed even though the field `remove_on_drop` is true.
    ///
    /// [`MmapFileMut::freeze`]: structs.MmapFileMut.html#methods.freeze
    #[inline]
    pub fn set_remove_on_drop(&mut self, val: bool) {
        self.remove_on_drop = val;
    }
}

impl_drop!(MmapFileMut, MmapFileMutInner, EmptyMmapFile);

