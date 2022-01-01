use std::borrow::Cow;
use std::mem;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use crate::tokio::{AsyncMmapFileReader, AsyncMmapFileWriter};
use crate::disk::tokio_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};
use crate::empty::tokio_impl::AsyncEmptyMmapFile;
use crate::error::{Error, Result};
use crate::memory::tokio_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut};
use crate::metadata::MetaData;
use crate::options::tokio_impl::AsyncOptions;

/// Utility methods to [`AsyncMmapFile`]
///
/// [`AsyncMmapFile`]: structs.AsyncMmapFile.html
#[async_trait]
#[enum_dispatch]
pub trait AsyncMmapFileExt {
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

    /// Whether the mmap is executable
    fn is_exec(&self) -> bool;

    /// Returns the metadata of file metadata
    ///
    /// Metadata information about a file.
    /// This structure is returned from the metadata or
    /// symlink_metadata function or method and represents
    /// known metadata about a file such as its permissions, size, modification times, etc
    async fn metadata(&self) -> Result<MetaData>;

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
    async fn write_all_to_new_file<P: AsRef<Path> + Send + Sync>(&self, new_file_path: P) -> Result<()> {
        let buf = self.as_slice();
        let opts = AsyncOptions::new().max_size(buf.len() as u64);

        let mut mmap = AsyncDiskMmapFileMut::create_with_options(new_file_path, opts).await?;
        mmap.writer(0)?.write_all(buf).await?;
        mmap.flush()
    }

    /// Write a range of content of the mmap file to new file.
    #[inline]
    async fn write_range_to_new_file<P: AsRef<Path> + Send + Sync>(&self, new_file_path: P, offset: usize, len: usize) -> Result<()> {
        let buf = self.as_slice();
        if buf.len() < offset + len {
            return Err(Error::EOF);
        }
        let opts = AsyncOptions::new().max_size(len as u64);

        let mut mmap = AsyncDiskMmapFileMut::create_with_options(new_file_path, opts).await?;
        mmap.writer(0)?.write_all(&buf[offset..offset + len]).await?;
        mmap.flush()
    }

    /// Returns a [`MmapFileReader`] which helps read data from mmap like a normal File.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    ///  `Err(Error::EOF)`.
    ///
    /// [`MmapFileReader`]: structs.MmapFileReader.html
    fn reader(&self, offset: usize) -> Result<AsyncMmapFileReader> {
        let buf = self.as_slice();
        if buf.len() < offset {
            Err(Error::EOF)
        } else {
            Ok(AsyncMmapFileReader::new(Cursor::new(&buf[offset..]), offset, buf.len() - offset))
        }
    }

    /// Returns a [`MmapFileReader`] base on the given `offset` and `len`, which helps read data from mmap like a normal File.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    ///  `Err(Error::EOF)`.
    ///
    /// [`MmapFileReader`]: structs.MmapFileReader.html
    fn range_reader(&self, offset: usize, len: usize) -> Result<AsyncMmapFileReader> {
        let buf = self.as_slice();
        if buf.len() < offset + len {
            Err(Error::EOF)
        } else {
            Ok(AsyncMmapFileReader::new(Cursor::new(&buf[offset.. offset + len]), offset, len))
        }
    }

    /// Locks the file for shared usage, blocking if the file is currently locked exclusively.
    fn lock_exclusive(&self) -> Result<()>;

    /// Locks the file for exclusive usage, blocking if the file is currently locked.
    fn lock_shared(&self) -> Result<()>;

    /// Locks the file for shared usage, or returns a an error if the file is currently locked (see lock_contended_error).
    fn try_lock_exclusive(&self) -> Result<()>;

    /// Locks the file for shared usage, or returns a an error if the file is currently locked (see lock_contended_error).Locks the file for shared usage, or returns a an error if the file is currently locked (see lock_contended_error).
    fn try_lock_shared(&self) -> Result<()>;

    /// Unlocks the file.
    fn unlock(&self) -> Result<()>;

    /// Read bytes to the dst buf from the offset, returns how many bytes read.
    fn read(&self, dst: &mut [u8], offset: usize) -> usize {
        let buf = self.as_slice();

        if buf.len() < offset {
            0
        } else {
            let remaining = buf.len() - offset;
            let dst_len = dst.len();
            if remaining > dst_len {
                dst.copy_from_slice(&buf[offset..offset + dst_len]);
                dst_len
            } else {
                dst.copy_from_slice(&buf[offset..offset + remaining]);
                remaining
            }
        }
    }

    /// Read the exact number of bytes required to fill buf.
    fn read_exact(&self, dst: &mut [u8], offset: usize) -> Result<()> {
        let buf = self.as_slice();
        let remaining = buf.len().checked_sub(offset);
        match remaining {
            None => Err(Error::EOF),
            Some(remaining) => {
                let dst_len = dst.len();
                if remaining < dst_len {
                    Err(Error::EOF)
                } else {
                    dst.copy_from_slice(&buf[offset..offset + dst_len]);
                    Ok(())
                }
            }
        }
    }

    /// Read a signed 8 bit integer from offset.
    fn read_i8(&self, offset: usize) -> Result<i8> {
        let buf = self.as_slice();

        let remaining = buf.len().checked_sub(offset);
        match remaining {
            None => Err(Error::EOF),
            Some(remaining) => {
                if remaining < 1 {
                    Err(Error::EOF)
                } else {
                    Ok(buf[offset] as i8)
                }
            }
        }
    }

    /// Read a signed 16 bit integer from offset in big-endian byte order.
    fn read_i16(&self, offset: usize) -> Result<i16> {
        read_impl!(self, offset, i16::from_be_bytes)
    }

    /// Read a signed 16 bit integer from offset in little-endian byte order.
    fn read_i16_le(&self, offset: usize) -> Result<i16> {
        read_impl!(self, offset, i16::from_le_bytes)
    }

    /// Read a signed integer from offset in big-endian byte order.
    fn read_isize(&self, offset: usize) -> Result<isize> {
        read_impl!(self, offset, isize::from_be_bytes)
    }

    /// Read a signed integer from offset in little-endian byte order.
    fn read_isize_le(&self, offset: usize) -> Result<isize> {
        read_impl!(self, offset, isize::from_le_bytes)
    }

    /// Read a signed 32 bit integer from offset in big-endian byte order.
    fn read_i32(&self, offset: usize) -> Result<i32> {
        read_impl!(self, offset, i32::from_be_bytes)
    }

    /// Read a signed 32 bit integer from offset in little-endian byte order.
    fn read_i32_le(&self, offset: usize) -> Result<i32> {
        read_impl!(self, offset, i32::from_le_bytes)
    }

    /// Read a signed 64 bit integer from offset in big-endian byte order.
    fn read_i64(&self, offset: usize) -> Result<i64> {
        read_impl!(self, offset, i64::from_be_bytes)
    }

    /// Read a signed 64 bit integer from offset in little-endian byte order.
    fn read_i64_le(&self, offset: usize) -> Result<i64> {
        read_impl!(self, offset, i64::from_le_bytes)
    }

    /// Read a signed 128 bit integer from offset in big-endian byte order.
    fn read_i128(&self, offset: usize) -> Result<i128> {
        read_impl!(self, offset, i128::from_be_bytes)
    }

    /// Read a signed 128 bit integer from offset in little-endian byte order.
    fn read_i128_le(&self, offset: usize) -> Result<i128> {
        read_impl!(self, offset, i128::from_le_bytes)
    }

    /// Read an unsigned 8 bit integer from offset.
    fn read_u8(&self, offset: usize) -> Result<u8> {
        let buf = self.as_slice();

        let remaining = buf.len().checked_sub(offset);
        match remaining {
            None => Err(Error::EOF),
            Some(remaining) => {
                if remaining < 1 {
                    Err(Error::EOF)
                } else {
                    Ok(buf[offset])
                }
            }
        }
    }

    /// Read an unsigned 16 bit integer from offset in big-endian.
    fn read_u16(&self, offset: usize) -> Result<u16> {
        read_impl!(self, offset, u16::from_be_bytes)
    }

    /// Read an unsigned 16 bit integer from offset in little-endian.
    fn read_u16_le(&self, offset: usize) -> Result<u16> {
        read_impl!(self, offset, u16::from_le_bytes)
    }

    /// Read an unsigned integer from offset in big-endian byte order.
    fn read_usize(&self, offset: usize) -> Result<usize> {
        read_impl!(self, offset, usize::from_be_bytes)
    }

    /// Read an unsigned integer from offset in little-endian byte order.
    fn read_usize_le(&self, offset: usize) -> Result<usize> {
        read_impl!(self, offset, usize::from_le_bytes)
    }

    /// Read an unsigned 32 bit integer from offset in big-endian.
    fn read_u32(&self, offset: usize) -> Result<u32> {
        read_impl!(self, offset, u32::from_be_bytes)
    }

    /// Read an unsigned 32 bit integer from offset in little-endian.
    fn read_u32_le(&self, offset: usize) -> Result<u32> {
        read_impl!(self, offset, u32::from_le_bytes)
    }

    /// Read an unsigned 64 bit integer from offset in big-endian.
    fn read_u64(&self, offset: usize) -> Result<u64> {
        read_impl!(self, offset, u64::from_be_bytes)
    }

    /// Read an unsigned 64 bit integer from offset in little-endian.
    fn read_u64_le(&self, offset: usize) -> Result<u64> {
        read_impl!(self, offset, u64::from_le_bytes)
    }

    /// Read an unsigned 128 bit integer from offset in big-endian.
    fn read_u128(&self, offset: usize) -> Result<u128> {
        read_impl!(self, offset, u128::from_be_bytes)
    }

    /// Read an unsigned 128 bit integer from offset in little-endian.
    fn read_u128_le(&self, offset: usize) -> Result<u128> {
        read_impl!(self, offset, u128::from_le_bytes)
    }

    /// Read an IEEE754 single-precision (4 bytes) floating point number from
    /// offset in big-endian byte order.
    fn read_f32(&self, offset: usize) -> Result<f32> {
        read_impl!(self, offset, f32::from_be_bytes)
    }

    /// Read an IEEE754 single-precision (4 bytes) floating point number from
    /// offset in little-endian byte order.
    fn read_f32_le(&self, offset: usize) -> Result<f32> {
        read_impl!(self, offset, f32::from_le_bytes)
    }

    /// Read an IEEE754 single-precision (8 bytes) floating point number from
    /// offset in big-endian byte order.
    fn read_f64(&self, offset: usize) -> Result<f64> {
        read_impl!(self, offset, f64::from_be_bytes)
    }

    /// Read an IEEE754 single-precision (8 bytes) floating point number from
    /// offset in little-endian byte order.
    fn read_f64_le(&self, offset: usize) -> Result<f64> {
        read_impl!(self, offset, f64::from_le_bytes)
    }
}

/// Utility methods to [`AsyncMmapFileMut`]
///
/// [`AsyncMmapFileMut`]: structs.AsyncMmapFileMut.html
#[async_trait]
#[enum_dispatch]
pub trait AsyncMmapFileMutExt {
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
        if buf.len() < offset + sz {
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
    async fn truncate(&mut self, max_sz: u64) -> Result<()>;

    /// Remove the underlying file
    async fn remove(self) -> Result<()>;

    /// Close and truncate the underlying file
    async fn close_with_truncate(self, max_sz: i64) -> Result<()>;

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
    fn writer(&mut self, offset: usize) -> Result<AsyncMmapFileWriter> {
        let buf = self.as_mut_slice();
        let buf_len = buf.len();
        if buf_len < offset {
            Err(Error::EOF)
        } else {
            Ok(AsyncMmapFileWriter::new(Cursor::new(&mut buf[offset..]), offset, buf_len - offset))
        }
    }

    /// Returns a [`AsyncMmapFileWriter`] base on the given `offset` and `len`, which helps read or write data from mmap like a normal File.
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
    /// [`flush`]: traits.AsyncMmapFileMutExt.html#methods.flush
    /// [`flush_range`]: traits.AsyncMmapFileMutExt.html#methods.flush_range
    /// [`flush_async`]: traits.AsyncMmapFileMutExt.html#methods.flush_async
    /// [`flush_async_range`]: traits.AsyncMmapFileMutExt.html#methods.flush_async_range
    /// [`AsyncMmapFileWriter`]: structs.AsyncMmapFileWriter.html
    fn range_writer(&mut self, offset: usize, len: usize) -> Result<AsyncMmapFileWriter> {
        let buf = self.as_mut_slice();
        if buf.len() < offset + len {
            Err(Error::EOF)
        } else {
            Ok(AsyncMmapFileWriter::new(
                Cursor::new(&mut buf[offset..offset + len]), offset, len))
        }
    }

    /// Write bytes to the mmap from the offset.
    fn write(&mut self, src: &[u8], offset: usize) -> usize {
        let buf = self.as_mut_slice();
        if buf.len() <= offset {
            0
        } else {
            let remaining = buf.len() - offset;
            let src_len = src.len();
            if remaining > src_len {
                buf[offset..offset + src_len].copy_from_slice(src);
                src_len
            } else {
                buf[offset..offset + remaining].copy_from_slice(&src[..remaining]);
                remaining
            }
        }
    }

    /// Write the all of bytes in `src` to the mmap from the offset.
    fn write_all(&mut self, src: &[u8], offset: usize) -> Result<()> {
        let buf = self.as_mut_slice();
        let remaining = buf.len().checked_sub(offset);
        match remaining {
            None => Err(Error::EOF),
            Some(remaining) => {
                let src_len = src.len();
                if remaining < src_len {
                    Err(Error::EOF)
                } else {
                    buf[offset..offset + src_len].copy_from_slice(src);
                    Ok(())
                }
            }
        }
    }

    /// Writes a signed 8 bit integer to mmap from the offset.
    fn write_i8(&mut self, val: i8, offset: usize) -> Result<()> {
        self.write_all(&[val as u8], offset)
    }

    /// Writes a signed 16 bit integer to mmap from the offset in the big-endian byte order.
    fn write_i16(&mut self, val: i16, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes a signed 16 bit integer to mmap from the offset in the little-endian byte order.
    fn write_i16_le(&mut self, val: i16, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }

    /// Writes a signed integer to mmap from the offset in the big-endian byte order.
    fn write_isize(&mut self, val: isize, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes a signed integer to mmap from the offset in the little-endian byte order.
    fn write_isize_le(&mut self, val: isize, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }

    /// Writes a signed 32 bit integer to mmap from the offset in the big-endian byte order.
    fn write_i32(&mut self, val: i32, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes a signed 32 bit integer to mmap from the offset in the little-endian byte order.
    fn write_i32_le(&mut self, val: i32, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }

    /// Writes a signed 64 bit integer to mmap from the offset in the big-endian byte order.
    fn write_i64(&mut self, val: i64, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes a signed 64 bit integer to mmap from the offset in the little-endian byte order.
    fn write_i64_le(&mut self, val: i64, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }

    /// Writes a signed 128 bit integer to mmap from the offset in the big-endian byte order.
    fn write_i128(&mut self, val: i128, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes a signed 128 bit integer to mmap from the offset in the little-endian byte order.
    fn write_i128_le(&mut self, val: i128, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }

    /// Writes an unsigned 8 bit integer to mmap from the offset.
    fn write_u8(&mut self, val: u8, offset: usize) -> Result<()> {
        self.write_all(&[val], offset)
    }

    /// Writes an unsigned 16 bit integer to mmap from the offset in the big-endian byte order.
    fn write_u16(&mut self, val: u16, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes an unsigned 16 bit integer to mmap from the offset in the little-endian byte order.
    fn write_u16_le(&mut self, val: u16, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }

    /// Writes an unsigned integer to mmap from the offset in the big-endian byte order.
    fn write_usize(&mut self, val: usize, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes an unsigned integer to mmap from the offset in the little-endian byte order.
    fn write_usize_le(&mut self, val: usize, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }

    /// Writes an unsigned 32 bit integer to mmap from the offset in the big-endian byte order.
    fn write_u32(&mut self, val: u32, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes an unsigned 32 bit integer to mmap from the offset in the little-endian byte order.
    fn write_u32_le(&mut self, val: u32, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }

    /// Writes an unsigned 64 bit integer to mmap from the offset in the big-endian byte order.
    fn write_u64(&mut self, val: u64, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes an unsigned 64 bit integer to mmap from the offset in the little-endian byte order.
    fn write_u64_le(&mut self, val: u64, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }

    /// Writes an unsigned 128 bit integer to mmap from the offset in the big-endian byte order.
    fn write_u128(&mut self, val: u128, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes an unsigned 128 bit integer to mmap from the offset in the little-endian byte order.
    fn write_u128_le(&mut self, val: u128, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }

    /// Writes an IEEE754 single-precision (4 bytes) floating point number to mmap from the offset in big-endian byte order.
    fn write_f32(&mut self, val: f32, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes an IEEE754 single-precision (4 bytes) floating point number to mmap from the offset in little-endian byte order.
    fn write_f32_le(&mut self, val: f32, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }

    /// Writes an IEEE754 single-precision (8 bytes) floating point number to mmap from the offset in big-endian byte order.
    fn write_f64(&mut self, val: f64, offset: usize) -> Result<()> {
        self.write_all(&val.to_be_bytes(), offset)
    }

    /// Writes an IEEE754 single-precision (8 bytes) floating point number to mmap from the offset in little-endian byte order.
    fn write_f64_le(&mut self, val: f64, offset: usize) -> Result<()> {
        self.write_all(&val.to_le_bytes(), offset)
    }
}

#[enum_dispatch(AsyncMmapFileExt)]
enum AsyncMmapFileInner {
    Empty(AsyncEmptyMmapFile),
    Memory(AsyncMemoryMmapFile),
    Disk(AsyncDiskMmapFile)
}

/// A read-only memory map file.
///
/// There is 3 status of this struct:
/// - __Disk__: mmap to a real file
/// - __Memory__: use [`Bytes`] to mock a mmap, which is useful for test and in-memory storage engine
/// - __Empty__: a state represents null mmap, which is helpful for drop, close the `AsyncMmapFile`. This state cannot be constructed directly.
///
/// [`Bytes`]: https://docs.rs/bytes/1.1.0/bytes/struct.Bytes.html
#[repr(transparent)]
pub struct AsyncMmapFile {
    inner: AsyncMmapFileInner
}

impl_async_mmap_file_ext!(AsyncMmapFile);

impl_from!(AsyncMmapFile, AsyncMmapFileInner, [AsyncEmptyMmapFile, AsyncMemoryMmapFile, AsyncDiskMmapFile]);

impl AsyncMmapFile {
    /// Open a readable memory map backed by a file
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFile, AsyncMmapFileExt};
    /// use fmmap::MetaDataExt;
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_open_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    /// // mmap the file
    /// let mut file = AsyncMmapFile::open("async_open_test.txt").await.unwrap();
    /// assert!(!file.is_empty());
    /// assert_eq!(file.metadata().await.unwrap().len(), 12);
    /// assert_eq!(file.len(), 12);
    /// assert_eq!(file.path_string(), String::from("async_open_test.txt"));
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFile::open(path).await?))
    }

    /// Open a readable memory map backed by a file with [`Options`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncOptions, AsyncMmapFile, AsyncMmapFileExt};
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_open_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_with_options_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    ///
    /// // mmap the file
    /// let opts = AsyncOptions::new()
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// // mmap the file
    /// let mut file = AsyncMmapFile::open_with_options("async_open_with_options_test.txt", opts).await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFile::open_with_options(path, opts).await?))
    }

    /// Open a readable and executable memory map backed by a file
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFile, AsyncMmapFileExt};
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_open_exec_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_exec_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    /// // mmap the file
    /// let mut file = AsyncMmapFile::open_exec("async_open_exec_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    pub async fn open_exec<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFile::open_exec(path).await?))
    }

    /// Open a readable and executable memory map backed by a file with [`Options`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFile, AsyncOptions, AsyncMmapFileExt};
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_open_exec_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_exec_with_options_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    ///
    /// // mmap the file
    /// let opts = AsyncOptions::new()
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// // mmap the file
    /// let mut file = AsyncMmapFile::open_exec_with_options("async_open_exec_with_options_test.txt", opts).await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open_exec_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFile::open_exec_with_options(path, opts).await?))
    }
}

impl_constructor_for_memory_mmap_file!(AsyncMemoryMmapFile, AsyncMmapFile, "AsyncMmapFile", "tokio::");

#[enum_dispatch(AsyncMmapFileExt, AsyncMmapFileMutExt)]
enum AsyncMmapFileMutInner {
    Empty(AsyncEmptyMmapFile),
    Memory(AsyncMemoryMmapFileMut),
    Disk(AsyncDiskMmapFileMut)
}

/// A writable memory map file.
///
/// There is 3 status of this struct:
/// - __Disk__: mmap to a real file
/// - __Memory__: use [`BytesMut`] to mock a mmap, which is useful for test and in-memory storage engine
/// - __Empty__: a state represents null mmap, which is helpful for drop, remove, close the `AsyncMmapFileMut`. This state cannot be constructed directly.
///
/// [`BytesMut`]: https://docs.rs/bytes/1.1.0/bytes/struct.BytesMut.html
pub struct AsyncMmapFileMut {
    inner: AsyncMmapFileMutInner,
    remove_on_drop: bool,
    deleted: bool,
}

impl_from_mut!(AsyncMmapFileMut, AsyncMmapFileMutInner, [AsyncEmptyMmapFile, AsyncMemoryMmapFileMut, AsyncDiskMmapFileMut]);

impl_async_mmap_file_ext!(AsyncMmapFileMut);

#[async_trait]
impl AsyncMmapFileMutExt for AsyncMmapFileMut {
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.inner.as_mut_slice()
    }

    #[inline]
    fn is_cow(&self) -> bool {
        self.inner.is_cow()
    }

    impl_flush!();

    #[inline]
    async fn truncate(&mut self, max_sz: u64) -> Result<()> {
        self.inner.truncate(max_sz).await
    }

    /// Remove the underlying file
    ///
    /// # Example
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFileMut, AsyncMmapFileMutExt};
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncMmapFileMut::create("async_disk_remove_test.txt").await.unwrap();
    ///
    /// file.truncate(12).await;
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.remove().await.unwrap();
    ///
    /// let err = tokio::fs::File::open("async_disk_remove_test.txt").await;
    /// assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    /// # })
    /// ```
    async fn remove(mut self) -> Result<()> {
        let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        if !self.remove_on_drop {
            // do remove
            inner.remove().await?;
            self.deleted = true;
        }
        Ok(())
    }

    /// Close and truncate the underlying file
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{MetaDataExt, tokio::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt}};
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncMmapFileMut::create("async_close_with_truncate_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_close_with_truncate_test.txt").unwrap());
    /// file.truncate(100).await;
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.close_with_truncate(50).await.unwrap();
    ///
    /// let file = AsyncMmapFileMut::open("async_close_with_truncate_test.txt").await.unwrap();
    /// let meta = file.metadata().await.unwrap();
    /// assert_eq!(meta.len(), 50);
    /// # })
    /// ```
    async fn close_with_truncate(mut self, max_sz: i64) -> Result<()> {
        let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        inner.close_with_truncate(max_sz).await
    }
}

impl AsyncMmapFileMut {
    /// Create a new file and mmap this file
    ///
    /// # Notes
    /// The new file is zero size, so, before write, you should truncate first.
    /// Or you can use [`create_with_options`] and set `max_size` field for [`AsyncOptions`] to enable directly write
    /// without truncating.
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFileExt, AsyncMmapFileMut, AsyncMmapFileMutExt};
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncMmapFileMut::create("async_create_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_create_test.txt").unwrap());
    /// assert!(file.is_empty());
    /// assert_eq!(file.path_string(), String::from("async_create_test.txt"));
    ///
    /// file.truncate(12).await;
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// # })
    /// ```
    ///
    /// [`create_with_options`]: struct.AsyncMmapFileMut.html#method.create_with_options
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFileMut::create(path).await?))
    }

    /// Create a new file and mmap this file with [`AsyncOptions`]
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFileMut, AsyncOptions, AsyncMmapFileMutExt, AsyncMmapFileExt};
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let opts = AsyncOptions::new()
    ///     // truncate to 100
    ///     .max_size(100);
    /// let mut file = AsyncMmapFileMut::create_with_options("async_create_with_options_test.txt", opts).await.unwrap();
    /// # defer!(std::fs::remove_file("async_create_with_options_test.txt").unwrap());
    /// assert!(!file.is_empty());
    ///
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn create_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFileMut::create_with_options(path, opts).await?))
    }

    /// Open or Create(if not exists) a file and mmap this file.
    ///
    /// # Notes
    /// If the file does not exist, then the new file will be open in zero size, so before do write, you should truncate first.
    /// Or you can use [`open_with_options`] and set `max_size` field for [`AsyncOptions`] to enable directly write
    /// without truncating.
    ///
    /// # Examples
    ///
    /// File already exists
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_open_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    ///
    /// // mmap the file
    /// let mut file = AsyncMmapFileMut::open("async_open_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate("some modified data...".len() as u64).await.unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("async_open_test.txt").await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// File does not exists
    ///
    /// ```no_run
    /// use fmmap::tokio::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // mmap the file
    /// let mut file = AsyncMmapFileMut::open("async_open_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_test.txt").unwrap());
    /// file.truncate(100).await.unwrap();
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate("some modified data...".len() as u64).await.unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("async_open_test.txt").await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`open_with_options`]: struct.AsyncMmapFileMut.html#method.open_with_options
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFileMut::open(path).await?))
    }

    /// Open or Create(if not exists) a file and mmap this file with [`AsyncOptions`].
    ///
    /// # Examples
    ///
    /// File already exists
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use tokio::fs::File;
    /// use std::io::SeekFrom;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_open_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_with_options_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    ///
    /// // mmap the file
    /// let opts = AsyncOptions::new()
    ///     // allow read
    ///     .read(true)
    ///     // allow write
    ///     .write(true)
    ///     // allow append
    ///     .append(true)
    ///     // truncate to 100
    ///     .max_size(100)
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// let mut file = AsyncMmapFileMut::open_with_options("async_open_with_options_test.txt", opts).await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).await.unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("async_open_with_options_test.txt").await.unwrap();
    /// // skip the sanity text
    /// tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start("sanity text".as_bytes().len() as u64)).await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// File does not exists
    ///
    /// ```no_run
    /// use fmmap::tokio::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // mmap the file with options
    /// let opts = AsyncOptions::new()
    ///     // allow read
    ///     .read(true)
    ///     // allow write
    ///     .write(true)
    ///     // allow append
    ///     .append(true)
    ///     // truncate to 100
    ///     .max_size(100);
    ///
    /// let mut file = AsyncMmapFileMut::open_with_options("async_open_with_options_test.txt", opts).await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_with_options_test.txt").unwrap());
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate("some modified data...".len() as u64).await.unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open(".txt").await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.htmlv
    pub async fn open_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFileMut::open_with_options(path, opts).await?))
    }

    /// Open an existing file and mmap this file
    ///
    /// # Examples
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // create a temp file
    /// let mut file = File::create("async_open_existing_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_existing_test.txt").unwrap());
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = AsyncMmapFileMut::open_exist("async_open_existing_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate("some modified data...".len() as u64).await.unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("async_open_existing_test.txt").await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFileMut::open_exist(path).await?))
    }

    /// Open an existing file and mmap this file with [`AsyncOptions`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use tokio::fs::File;
    /// use std::io::SeekFrom;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // create a temp file
    /// let mut file = File::create("async_open_existing_test_with_options.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_existing_test_with_options.txt").unwrap());
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let opts = AsyncOptions::new()
    ///     // truncate to 100
    ///     .max_size(100)
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    ///
    /// let mut file = AsyncMmapFileMut::open_exist_with_options("async_open_existing_test_with_options.txt", opts).await.unwrap();
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).await.unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    ///
    /// // reopen to check content, cow will not change the content.
    /// let mut file = File::open("async_open_existing_test_with_options.txt").await.unwrap();
    /// let mut buf = vec![0; "some modified data...".len()];
    /// // skip the sanity text
    /// tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start("sanity text".as_bytes().len() as u64)).await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFileMut::open_exist_with_options(path, opts).await?))
    }

    /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file).
    /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt};
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // create a temp file
    /// let mut file = File::create("async_open_cow_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_cow_test.txt").unwrap());
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = AsyncMmapFileMut::open_cow("async_open_cow_test.txt").await.unwrap();
    /// assert!(file.is_cow());
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.write_all("some data!!!".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// // cow, change will only be seen in current caller
    /// assert_eq!(file.as_slice(), "some data!!!".as_bytes());
    /// drop(file);
    ///
    /// // reopen to check content, cow will not change the content.
    /// let mut file = File::open("async_open_cow_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFileMut::open_cow(path).await?))
    }

    /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file) with [`AsyncOptions`].
    /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use tokio::fs::File;
    /// use std::io::SeekFrom;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // create a temp file
    /// let mut file = File::create("async_open_cow_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_cow_with_options_test.txt").unwrap());
    ///
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let opts = AsyncOptions::new()
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    ///
    /// let mut file = AsyncMmapFileMut::open_cow_with_options("async_open_cow_with_options_test.txt", opts).await.unwrap();
    /// assert!(file.is_cow());
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.write_all("some data!!!".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// // cow, change will only be seen in current caller
    /// assert_eq!(file.as_slice(), "some data!!!".as_bytes());
    /// drop(file);
    ///
    /// // reopen to check content, cow will not change the content.
    /// let mut file = File::open("async_open_cow_with_options_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// // skip the sanity text
    /// tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start("sanity text".as_bytes().len() as u64)).await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open_cow_with_options<P: AsRef<Path>>(path: P, opts: AsyncOptions) -> Result<Self> {
        Ok(Self::from(AsyncDiskMmapFileMut::open_cow_with_options(path, opts).await?))
    }

    /// Make the mmap file read-only.
    ///
    /// # Notes
    /// If `remove_on_drop` is set to `true`, then the underlying file will not be removed on drop if this function is invoked. [Read more]
    ///
    /// Returns an immutable version of this memory mapped buffer.
    /// If the memory map is file-backed, the file must have been opened with read permissions.
    ///
    /// # Errors
    /// This method returns an error when the underlying system call fails,
    /// which can happen for a variety of reasons,
    /// such as when the file has not been opened with read permissions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFileMut, AsyncMmapFileMutExt};
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncMmapFileMut::create("async_disk_freeze_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_disk_freeze_test.txt").unwrap());
    /// file.truncate(12).await;
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// // freeze
    /// file.freeze().unwrap();
    /// # })
    /// ```
    ///
    /// [Read more]: structs.AsyncMmapFileMut.html#methods.set_remove_on_drop
    #[inline]
    pub fn freeze(mut self) -> Result<AsyncMmapFile> {
        let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        match inner {
            AsyncMmapFileMutInner::Empty(empty) => Ok(AsyncMmapFile::from(empty)), // unreachable, keep this for good measure
            AsyncMmapFileMutInner::Memory(memory) => Ok(AsyncMmapFile::from(memory.freeze())),
            AsyncMmapFileMutInner::Disk(disk) => Ok(AsyncMmapFile::from(disk.freeze()?)),
        }
    }

    /// Transition the memory map to be readable and executable.
    /// If the memory map is file-backed, the file must have been opened with execute permissions.
    ///
    /// # Notes
    /// If `remove_on_drop` is set to `true`, then the underlying file will not be removed on drop if this function is invoked. [Read more]
    ///
    /// # Errors
    /// This method returns an error when the underlying system call fails,
    /// which can happen for a variety of reasons,
    /// such as when the file has not been opened with execute permissions
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::tokio::{AsyncMmapFileExt, AsyncMmapFileMut, AsyncMmapFileMutExt};
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncMmapFileMut::create("async_freeze_exec_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_freeze_exec_test.txt").unwrap());
    /// file.truncate(12).await;
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// // freeze_exec
    /// let file = file.freeze_exec().unwrap();
    /// assert!(file.is_exec());
    /// # })
    /// ```
    ///
    /// [Read more]: structs.AsyncMmapFileMut.html#methods.set_remove_on_drop
    #[inline]
    pub fn freeze_exec(mut self) -> Result<AsyncMmapFile> {
        let empty = AsyncMmapFileMutInner::Empty(AsyncEmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        match inner {
            AsyncMmapFileMutInner::Empty(empty) => Ok(AsyncMmapFile::from(empty)), // unreachable, keep this for good measure
            AsyncMmapFileMutInner::Memory(memory) => Ok(AsyncMmapFile::from(memory.freeze())),
            AsyncMmapFileMutInner::Disk(disk) => Ok(AsyncMmapFile::from(disk.freeze_exec()?)),
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
    /// If invoke [`AsyncMmapFileMut::freeze`], then the file will
    /// not be removed even though the field `remove_on_drop` is true.
    ///
    /// [`AsyncMmapFileMut::freeze`]: structs.AsyncMmapFileMut.html#methods.freeze
    #[inline]
    pub fn set_remove_on_drop(&mut self, val: bool) {
        self.remove_on_drop = val;
    }
}

impl_constructor_for_memory_mmap_file_mut!(AsyncMemoryMmapFileMut, AsyncMmapFileMut, "AsyncMmapFileMut", "tokio::");

impl_drop!(AsyncMmapFileMut, AsyncMmapFileMutInner, AsyncEmptyMmapFile);
