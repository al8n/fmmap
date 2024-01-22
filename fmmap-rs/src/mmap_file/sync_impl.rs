use crate::disk::{DiskMmapFile, DiskMmapFileMut};
use crate::empty::EmptyMmapFile;
use crate::error::{Error, ErrorKind, Result};
use crate::memory::{MemoryMmapFile, MemoryMmapFileMut};
use crate::metadata::MetaData;
use crate::options::Options;
use crate::{MmapFileReader, MmapFileWriter};
use std::borrow::Cow;
use std::io::{Cursor, Write};
use std::mem;
use std::path::{Path, PathBuf};

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
        &self.as_slice()[offset..offset + sz]
    }

    /// bytes returns data starting from offset off of size sz.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    /// `Err(Error::from(ErrorKind::EOF))`.
    fn bytes(&self, offset: usize, sz: usize) -> Result<&[u8]> {
        let buf = self.as_slice();
        if buf.len() < offset + sz {
            Err(Error::from(ErrorKind::EOF))
        } else {
            Ok(&buf[offset..offset + sz])
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
        let opts = Options::new().max_size(buf.len() as u64);
        let mut mmap = DiskMmapFileMut::create_with_options(new_file_path, opts)?;
        mmap.writer(0)?.write_all(buf)?;
        mmap.flush()
    }

    /// Write a range of content of the mmap file to new file.
    #[inline]
    fn write_range_to_new_file<P: AsRef<Path>>(
        &self,
        new_file_path: P,
        offset: usize,
        len: usize,
    ) -> Result<()> {
        let buf = self.as_slice();
        if buf.len() < offset + len {
            return Err(Error::from(ErrorKind::EOF));
        }
        let opts = Options::new().max_size(len as u64);
        let mut mmap = DiskMmapFileMut::create_with_options(new_file_path, opts)?;
        mmap.writer(0)?.write_all(&buf[offset..offset + len])?;
        mmap.flush()
    }

    /// Returns a [`MmapFileReader`] which helps read data from mmap like a normal File.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    ///  `Err(Error::from(ErrorKind::EOF))`.
    ///
    /// [`MmapFileReader`]: structs.MmapFileReader.html
    fn reader(&self, offset: usize) -> Result<MmapFileReader> {
        let buf = self.as_slice();
        if buf.len() < offset {
            Err(Error::from(ErrorKind::EOF))
        } else {
            Ok(MmapFileReader::new(
                Cursor::new(&buf[offset..]),
                offset,
                buf.len() - offset,
            ))
        }
    }

    /// Returns a [`MmapFileReader`] base on the given `offset` and `len`, which helps read data from mmap like a normal File.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    ///  `Err(Error::from(ErrorKind::EOF))`.
    ///
    /// [`MmapFileReader`]: structs.MmapFileReader.html
    fn range_reader(&self, offset: usize, len: usize) -> Result<MmapFileReader> {
        let buf = self.as_slice();
        if buf.len() < offset + len {
            Err(Error::from(ErrorKind::EOF))
        } else {
            Ok(MmapFileReader::new(
                Cursor::new(&buf[offset..offset + len]),
                offset,
                len,
            ))
        }
    }

    /// Locks the file for exclusively usage, blocking if the file is currently locked.
    ///
    /// # Notes
    /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
    fn lock_exclusive(&self) -> Result<()>;

    /// Locks the file for shared usage, blocking if the file is currently locked exclusively.
    ///
    /// # Notes
    /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
    fn lock_shared(&self) -> Result<()>;

    /// Locks the file for exclusively usage, or returns a an error if the file is currently locked (see lock_contended_error).
    ///
    /// # Notes
    /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
    fn try_lock_exclusive(&self) -> Result<()>;

    /// Locks the file for shared usage, or returns a an error if the file is currently locked exclusively (see lock_contended_error).
    ///
    /// # Notes
    /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
    fn try_lock_shared(&self) -> Result<()>;

    /// Unlocks the file.
    ///
    /// # Notes
    /// This function will do nothing if the underlying is not a real file, e.g. in-memory.
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
            None => Err(Error::from(ErrorKind::EOF)),
            Some(remaining) => {
                let dst_len = dst.len();
                if remaining < dst_len {
                    Err(Error::from(ErrorKind::EOF))
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
            None => Err(Error::from(ErrorKind::EOF)),
            Some(remaining) => {
                if remaining < 1 {
                    Err(Error::from(ErrorKind::EOF))
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
            None => Err(Error::from(ErrorKind::EOF)),
            Some(remaining) => {
                if remaining < 1 {
                    Err(Error::from(ErrorKind::EOF))
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
        &mut self.as_mut_slice()[offset..offset + sz]
    }

    /// Whether mmap is copy on write
    fn is_cow(&self) -> bool;

    /// bytes_mut returns mutable data starting from offset off of size sz.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    /// `Err(Error::from(ErrorKind::EOF))`.
    fn bytes_mut(&mut self, offset: usize, sz: usize) -> Result<&mut [u8]> {
        let buf = self.as_mut_slice();
        if buf.len() <= offset + sz {
            Err(Error::from(ErrorKind::EOF))
        } else {
            Ok(&mut buf[offset..offset + sz])
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
    fn drop_remove(self) -> Result<()>;

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
    ///  `Err(Error::from(ErrorKind::EOF))`.
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
            Err(Error::from(ErrorKind::EOF))
        } else {
            Ok(MmapFileWriter::new(
                Cursor::new(&mut buf[offset..]),
                offset,
                buf_len - offset,
            ))
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
    ///  `Err(Error::from(ErrorKind::EOF))`.
    ///
    /// [`flush`]: traits.MmapFileMutExt.html#methods.flush
    /// [`flush_range`]: traits.MmapFileMutExt.html#methods.flush_range
    /// [`flush_async`]: traits.MmapFileMutExt.html#methods.flush_async
    /// [`flush_async_range`]: traits.MmapFileMutExt.html#methods.flush_async_range
    /// [`MmapFileWriter`]: structs.MmapFileWriter.html
    fn range_writer(&mut self, offset: usize, len: usize) -> Result<MmapFileWriter> {
        let buf = self.as_mut_slice();
        if buf.len() < offset + len {
            Err(Error::from(ErrorKind::EOF))
        } else {
            Ok(MmapFileWriter::new(
                Cursor::new(&mut buf[offset..offset + len]),
                offset,
                len,
            ))
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
            None => Err(Error::from(ErrorKind::EOF)),
            Some(remaining) => {
                let src_len = src.len();
                if remaining < src_len {
                    Err(Error::from(ErrorKind::EOF))
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

#[enum_dispatch(MmapFileExt)]
enum MmapFileInner {
    Empty(EmptyMmapFile),
    Memory(MemoryMmapFile),
    Disk(DiskMmapFile),
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

impl_from!(
    MmapFile,
    MmapFileInner,
    [EmptyMmapFile, MemoryMmapFile, DiskMmapFile]
);

impl MmapFile {
    /// Open a readable memory map backed by a file
    ///
    /// # Examples
    ///
    /// ```no_compile
    /// use fmmap::{MmapFile, MmapFileExt};
    /// use std::fs::{remove_file, File};
    /// use std::io::Write;
    /// # use scopeguard::defer;
    ///
    /// # let mut file = File::create("open_test.txt").unwrap();
    /// # defer!(remove_file("open_test.txt").unwrap());
    /// # file.write_all("some data...".as_bytes()).unwrap();
    /// # drop(file);
    ///
    /// // open and mmap the file
    /// let mut file = MmapFile::open("open_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(DiskMmapFile::open(path)?))
    }

    /// Open a readable memory map backed by a file with [`Options`]
    ///
    /// # Examples
    ///
    /// ```no_compile
    /// use fmmap::{Options, MmapFile, MmapFileExt};
    /// # use scopeguard::defer;
    ///
    /// # let mut file = std::fs::File::create("open_test_with_options.txt").unwrap();
    /// # defer!(std::fs::remove_file("open_test_with_options.txt").unwrap());
    /// # std::io::Write::write_all(&mut file, "sanity text".as_bytes()).unwrap();
    /// # std::io::Write::write_all(&mut file, "some data...".as_bytes()).unwrap();
    /// # drop(file);
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
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
    /// // open and mmap the file
    /// let mut file = MmapFile::open_with_options("open_test_with_options.txt", opts).unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
        Ok(Self::from(DiskMmapFile::open_with_options(path, opts)?))
    }

    /// Open a readable memory map backed by a file
    ///
    /// # Examples
    ///
    /// ```no_compile
    /// use fmmap::{MmapFile, MmapFileExt};
    /// use std::fs::{remove_file, File};
    /// use std::io::Write;
    /// # use scopeguard::defer;
    ///
    /// # let mut file = File::create("open_exec_test.txt").unwrap();
    /// # defer!(remove_file("open_exec_test.txt").unwrap());
    /// # file.write_all("some data...".as_bytes()).unwrap();
    /// # drop(file);
    ///
    /// // open and mmap the file
    /// let mut file = MmapFile::open_exec("open_exec_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    pub fn open_exec<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(DiskMmapFile::open_exec(path)?))
    }

    /// Open a readable and executable memory map backed by a file with [`Options`].
    ///
    /// # Examples
    ///
    /// ```no_compile
    /// use fmmap::{Options, MmapFile, MmapFileExt};
    /// # use scopeguard::defer;
    ///
    /// # let mut file = std::fs::File::create("open_exec_test_with_options.txt").unwrap();
    /// # defer!(std::fs::remove_file("open_exec_test_with_options.txt").unwrap());
    /// # std::io::Write::write_all(&mut file, "sanity text".as_bytes()).unwrap();
    /// # std::io::Write::write_all(&mut file, "some data...".as_bytes()).unwrap();
    /// # drop(file);
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
    ///     // allow read
    ///     .read(true)
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// // open and mmap the file
    /// let mut file = MmapFile::open_exec_with_options("open_exec_test_with_options.txt", opts).unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_exec_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
        Ok(Self::from(DiskMmapFile::open_exec_with_options(
            path, opts,
        )?))
    }
}

impl_constructor_for_memory_mmap_file!(MemoryMmapFile, MmapFile, "MmapFile", "sync");

#[enum_dispatch(MmapFileExt, MmapFileMutExt)]
enum MmapFileMutInner {
    Empty(EmptyMmapFile),
    Memory(MemoryMmapFileMut),
    Disk(DiskMmapFileMut),
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

impl_from_mut!(
    MmapFileMut,
    MmapFileMutInner,
    [EmptyMmapFile, MemoryMmapFileMut, DiskMmapFileMut]
);

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

    /// Remove the underlying file
    ///
    /// # Examples
    ///
    /// ```no_compile
    /// use fmmap::{MmapFileMut, MmapFileMutExt};
    /// # use scopeguard::defer;
    ///
    /// let mut file = MmapFileMut::create("remove_test.txt").unwrap();
    ///
    /// file.truncate(12);
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.drop_remove().unwrap();
    ///
    /// let err = std::fs::File::open("remove_test.txt");
    /// assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    /// ```
    fn drop_remove(mut self) -> Result<()> {
        let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        if !self.remove_on_drop {
            // do remove
            inner.drop_remove()?;
            self.deleted = true;
        }
        Ok(())
    }

    /// Close and truncate the underlying file
    ///
    /// # Examples
    ///
    /// ```no_compile
    /// use fmmap::{MetaDataExt, MmapFileMut, MmapFileExt, MmapFileMutExt};
    /// # use scopeguard::defer;
    ///
    /// let mut file = MmapFileMut::create("close_with_truncate_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("close_with_truncate_test.txt").unwrap());
    /// file.truncate(12);
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.close_with_truncate(50).unwrap();
    ///
    /// let file = MmapFileMut::open("close_with_truncate_test.txt").unwrap();
    /// let meta = file.metadata().unwrap();
    /// assert_eq!(meta.len(), 50);
    /// ```
    fn close_with_truncate(mut self, max_sz: i64) -> Result<()> {
        let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        inner.close_with_truncate(max_sz)
    }
}

impl MmapFileMut {
    /// Create a new file and mmap this file
    ///
    /// # Notes
    /// The new file is zero size, so before do write, you should truncate first.
    /// Or you can use [`Options::create_mmap_file_mut`] and set `max_size` field for [`Options`] to enable directly write
    /// without truncating.
    ///
    /// # Examples
    ///
    /// ```no_compile
    /// use fmmap::{Options, MmapFileMut, MmapFileMutExt, MmapFileExt};
    /// # use scopeguard::defer;
    ///
    /// let mut file = MmapFileMut::create("create_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("create_test.txt").unwrap());
    /// assert!(file.is_empty());
    /// assert_eq!(file.path_string(), String::from("create_test.txt"));
    ///
    /// file.truncate(12);
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// ```
    ///
    /// [`Options::create_mmap_file_mut`]: struct.Options.html#method.create_mmap_file_mut
    /// [`Options`]: struct.Options.html
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(DiskMmapFileMut::create(path)?))
    }

    /// Create a new file and mmap this file with [`Options`]
    ///
    /// # Examples
    ///
    /// ```no_compile
    /// use fmmap::{Options, MmapFileMut, MmapFileMutExt, MmapFileExt};
    /// # use scopeguard::defer;
    ///
    /// let opts = Options::new()
    ///     // truncate to 100
    ///     .max_size(100);
    /// let mut file = MmapFileMut::create_with_options("create_with_options_test.txt", opts).unwrap();
    /// # defer!(std::fs::remove_file("create_with_options_test.txt").unwrap());
    /// assert!(!file.is_empty());
    ///
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn create_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
        Ok(Self::from(DiskMmapFileMut::create_with_options(
            path, opts,
        )?))
    }

    /// Open or Create(if not exists) a file and mmap this file.
    ///
    /// # Notes
    /// If the file does not exist, then the new file will be open in zero size, so before do write, you should truncate first.
    /// Or you can use [`open_with_options`] and set `max_size` field for [`Options`] to enable directly write
    /// without truncating.
    ///
    /// # Examples
    ///
    /// File already exists
    ///
    /// ```no_compile
    /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt};
    /// use std::fs::File;
    /// use std::io::{Read, Write};
    /// # use scopeguard::defer;
    ///
    /// # let mut file = File::create("open_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("open_test.txt").unwrap());
    /// # file.write_all("some data...".as_bytes()).unwrap();
    /// # drop(file);
    ///
    /// // open and mmap the file
    /// let mut file = MmapFileMut::open("open_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate("some modified data...".len() as u64).unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("open_test.txt").unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// File does not exists
    ///
    /// ```no_run
    /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt};
    /// use std::fs::{remove_file, File};
    /// use std::io::{Read, Write};
    /// # use scopeguard::defer;
    ///
    /// // create and mmap the file
    /// let mut file = MmapFileMut::open("open_test.txt").unwrap();
    /// # defer!(remove_file("open_test.txt").unwrap());
    /// file.truncate(100).unwrap();
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate("some modified data...".len() as u64).unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("open_test.txt").unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// [`open_with_options`]: struct.MmapFileMut.html#method.open_with_options
    /// [`Options`]: struct.Options.html
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(DiskMmapFileMut::open(path)?))
    }

    /// Open or Create(if not exists) a file and mmap this file with [`Options`].
    ///
    /// # Examples
    ///
    /// File already exists
    ///
    /// ```no_compile
    /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt, Options};
    /// use std::fs::{remove_file, File};
    /// use std::io::{Read, Seek, SeekFrom, Write};
    /// # use scopeguard::defer;
    ///
    /// # let mut file = File::create("open_test_with_options.txt").unwrap();
    /// # defer!(remove_file("open_test_with_options.txt").unwrap());
    /// # file.write_all("sanity text".as_bytes()).unwrap();
    /// # file.write_all("some data...".as_bytes()).unwrap();
    /// # drop(file);
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
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
    /// let mut file = MmapFileMut::open_with_options("open_test_with_options.txt", opts).unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("open_test_with_options.txt").unwrap();
    /// file.seek(SeekFrom::Start("sanity text".as_bytes().len() as u64)).unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// File does not exists
    ///
    /// ```no_run
    /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt, Options};
    /// use std::fs::File;
    /// use std::io::{Read, Write};
    /// # use scopeguard::defer;
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
    ///     // allow read
    ///     .read(true)
    ///     // allow write
    ///     .write(true)
    ///     // allow append
    ///     .append(true)
    ///     // truncate to 100
    ///     .max_size(100);
    ///
    /// let mut file = MmapFileMut::open_with_options("open_test_with_options.txt", opts).unwrap();
    /// # defer!(std::fs::remove_file("open_test_with_options.txt").unwrap());
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate("some modified data...".len() as u64).unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("open_test_with_options.txt").unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
        Ok(Self::from(DiskMmapFileMut::open_with_options(path, opts)?))
    }

    /// Open an existing file and mmap this file
    ///
    /// # Examples
    /// ```no_compile
    /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt};
    /// use std::fs::File;
    /// use std::io::{Read, Write};
    /// # use scopeguard::defer;
    ///
    /// // create a temp file
    /// let mut file = File::create("open_existing_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("open_existing_test.txt").unwrap());
    /// file.write_all("some data...".as_bytes()).unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = MmapFileMut::open_exist("open_existing_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate("some modified data...".len() as u64).unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("open_existing_test.txt").unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    pub fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(DiskMmapFileMut::open_exist(path)?))
    }

    /// Open an existing file and mmap this file with [`Options`]
    ///
    /// # Examples
    /// ```no_compile
    /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt, Options};
    /// use std::fs::File;
    /// use std::io::{Read, Seek, SeekFrom, Write};
    /// # use scopeguard::defer;
    ///
    /// // create a temp file
    /// let mut file = File::create("open_existing_test_with_options.txt").unwrap();
    /// # defer!(std::fs::remove_file("open_existing_test_with_options.txt").unwrap());
    /// file.write_all("sanity text".as_bytes()).unwrap();
    /// file.write_all("some data...".as_bytes()).unwrap();
    /// drop(file);
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
    ///     // truncate to 100
    ///     .max_size(100)
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// let mut file = MmapFileMut::open_exist_with_options("open_existing_test_with_options.txt", opts).unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("open_existing_test_with_options.txt").unwrap();
    /// file.seek(SeekFrom::Start("sanity text".as_bytes().len() as u64)).unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
        Ok(Self::from(DiskMmapFileMut::open_exist_with_options(
            path, opts,
        )?))
    }

    /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file).
    /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
    ///
    /// # Examples
    ///
    /// ```no_compile
    /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt};
    /// use std::fs::File;
    /// use std::io::{Read, Write};
    /// # use scopeguard::defer;
    ///
    /// // create a temp file
    /// let mut file = File::create("open_cow_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("open_cow_test.txt").unwrap());
    /// file.write_all("some data...".as_bytes()).unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = MmapFileMut::open_cow("open_cow_test.txt").unwrap();
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
    /// let mut file = File::open("open_cow_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from(DiskMmapFileMut::open_cow(path)?))
    }

    /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file) with [`Options`].
    /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
    ///
    /// # Examples
    ///
    /// ```no_compile
    /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt, Options};
    /// use std::fs::File;
    /// use std::io::{Read, Seek, Write, SeekFrom};
    /// # use scopeguard::defer;
    ///
    /// // create a temp file
    /// let mut file = File::create("open_cow_with_options_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("open_cow_with_options_test.txt").unwrap());
    /// file.write_all("sanity text".as_bytes()).unwrap();
    /// file.write_all("some data...".as_bytes()).unwrap();
    /// drop(file);
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// let mut file = MmapFileMut::open_cow_with_options("open_cow_with_options_test.txt", opts).unwrap();
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
    /// let mut file = File::open("open_cow_with_options_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// // skip the sanity text
    /// file.seek(SeekFrom::Start("sanity text".as_bytes().len() as u64)).unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_cow_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self> {
        Ok(Self::from(DiskMmapFileMut::open_cow_with_options(
            path, opts,
        )?))
    }

    /// Make the mmap file read-only.
    ///
    /// # Notes
    /// If `remove_on_drop` is set to `true`, then the underlying file will not be removed on drop if this function is invoked. [Read more]
    ///
    /// # Examples
    /// ```no_compile
    /// use fmmap::{MmapFileMut, MmapFileMutExt};
    /// # use scopeguard::defer;
    ///
    /// let mut file = MmapFileMut::create("mmap_file_freeze_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("mmap_file_freeze_test.txt").unwrap());
    /// file.truncate(12);
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.freeze().unwrap();
    /// ```
    ///
    /// [Read more]: structs.MmapFileMut.html#methods.set_remove_on_drop
    pub fn freeze(mut self) -> Result<MmapFile> {
        let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        match inner {
            MmapFileMutInner::Empty(empty) => Ok(MmapFile::from(empty)), // unreachable, keep this for good measure
            MmapFileMutInner::Memory(memory) => Ok(MmapFile::from(memory.freeze())),
            MmapFileMutInner::Disk(disk) => Ok(MmapFile::from(disk.freeze()?)),
        }
    }

    /// Transition the memory map to be readable and executable.
    /// If the memory map is file-backed, the file must have been opened with execute permissions.
    ///
    /// # Errors
    /// This method returns an error when the underlying system call fails,
    /// which can happen for a variety of reasons,
    /// such as when the file has not been opened with execute permissions
    ///
    /// # Examples
    /// ```no_compile
    /// use fmmap::{MmapFileExt, MmapFileMut, MmapFileMutExt};
    /// # use scopeguard::defer;
    ///
    /// let mut file = MmapFileMut::create("mmap_file_freeze_exec_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("mmap_file_freeze_exec_test.txt").unwrap());
    /// file.truncate(12);
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// let file = file.freeze_exec().unwrap();
    /// assert!(file.is_exec());
    /// ```
    pub fn freeze_exec(mut self) -> Result<MmapFile> {
        let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        match inner {
            MmapFileMutInner::Empty(empty) => Ok(MmapFile::from(empty)), // unreachable, keep this for good measure
            MmapFileMutInner::Memory(memory) => Ok(MmapFile::from(memory.freeze())),
            MmapFileMutInner::Disk(disk) => Ok(MmapFile::from(disk.freeze_exec()?)),
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

    /// Close the file. It would also truncate the file if max_sz >= 0.
    #[inline]
    pub fn close(&mut self, max_sz: i64) -> Result<()> {
        let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        match inner {
            MmapFileMutInner::Disk(disk) => disk.flush().and_then(|_| {
                if max_sz >= 0 {
                    disk.file.set_len(max_sz as u64).map_err(From::from)
                } else {
                    Ok(())
                }
            }),
            _ => Ok(()),
        }
    }

    /// Remove the underlying file without dropping, leaving an [`EmptyMmapFile`].
    #[inline]
    pub fn remove(&mut self) -> Result<()> {
        let empty = MmapFileMutInner::Empty(EmptyMmapFile::default());
        // swap the inner to empty
        let inner = mem::replace(&mut self.inner, empty);
        match inner {
            MmapFileMutInner::Disk(disk) => {
                let path = disk.path;
                drop(disk.mmap);
                disk.file
                    .set_len(0)
                    .and_then(|_| {
                        drop(disk.file);
                        std::fs::remove_file(path)
                    })
                    .map_err(From::from)
            }
            _ => Ok(()),
        }
    }
}

impl_constructor_for_memory_mmap_file_mut!(MemoryMmapFileMut, MmapFileMut, "MmapFileMut", "sync");

impl_drop!(MmapFileMut, MmapFileMutInner, EmptyMmapFile);

impl_sync_tests!("", MmapFile, MmapFileMut);
