use std::borrow::Cow;
use std::mem;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use crate::{AsyncMmapFileReader, AsyncMmapFileWriter};
use crate::error::{Error, Result};
use crate::metadata::MetaData;


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
        if buf.len() <= offset + sz {
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

    async fn stat(&self) -> Result<MetaData>;

    /// Returns a [`MmapFileReader`] which helps read data from mmap like a normal File.
    ///
    /// # Errors
    /// If there's not enough data, it would return
    ///  `Err(Error::EOF)`.
    ///
    /// [`MmapFileReader`]: structs.MmapFileReader.html
    fn reader(&self, offset: usize) -> Result<AsyncMmapFileReader> {
        let buf = self.as_slice();
        if buf.len() <= offset {
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
    fn reader_range(&self, offset: usize, len: usize) -> Result<AsyncMmapFileReader> {
        let buf = self.as_slice();
        if buf.len() <= offset + len {
            Err(Error::EOF)
        } else {
            Ok(AsyncMmapFileReader::new(Cursor::new(&buf[offset.. offset + len]), offset, len))
        }
    }

    /// Read bytes to the dst buf from the offset, returns how many bytes read.
    fn read(&self, dst: &mut [u8], offset: usize) -> usize {
        let buf = self.as_slice();

        if buf.len() <= offset {
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
        let remaining = buf.len().checked_sub( offset);
        match remaining {
            None => Err(Error::EOF),
            Some(remaining) => {
                let dst_len = dst.len();
                if remaining < dst_len {
                    Err(Error::EOF)
                } else {
                    dst.copy_from_slice(&buf[offset .. offset + dst_len]);
                    Ok(())
                }
            }
        }
    }

    /// Read a signed 8 bit integer from offset.
    fn read_i8(&self, offset: usize) -> Result<i8> {
        let buf = self.as_slice();

        let remaining = buf.len().checked_sub( offset);
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

        let remaining = buf.len().checked_sub( offset);
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
    async fn truncate(&mut self, max_sz: u64) -> Result<()>;

    async fn delete(self) -> Result<()>;

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
        if buf_len <= offset {
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
    fn writer_range(&mut self, offset: usize, len: usize) -> Result<AsyncMmapFileWriter> {
        let buf = self.as_mut_slice();
        if buf.len() <= offset + len {
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
                Ok(src_len)
            } else {
                buf[offset..offset + remaining].copy_from_slice(&src[..remaining]);
                Ok(remaining)
            }
        }
    }

    /// Write the all of bytes in `src` to the mmap from the offset.
    fn write_all(&mut self, src: &[u8], offset: usize) -> Result<()> {
        let buf = self.as_mut_slice();
        let remaining = buf.len().checked_sub( offset);
        match remaining {
            None => Err(Error::EOF),
            Some(remaining) => {
                let src_len = src.len();
                if remaining < src_len {
                    Err(Error::EOF)
                } else {
                    buf[offset .. offset + src_len].copy_from_slice(src);
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
