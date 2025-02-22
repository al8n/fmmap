use std::fmt::{Debug, Formatter};
use std::io;
use std::io::Write;
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use bytes::Buf;


/// MmapFileWriter helps read or write data from mmap file
/// like a normal file.
///
/// # Notes
/// If you use a writer to write data to mmap, there is no guarantee all
/// data will be durably stored. So you need to call [`flush`]/[`flush_range`]/[`flush_async`]/[`flush_async_range`] in [`MmapFileMutExt`]
/// to guarantee all data will be durably stored.
///
/// [`flush`]: trait.MmapFileMutExt.html#methods.flush
/// [`flush_range`]: trait.MmapFileMutExt.html#methods.flush_range
/// [`flush_async`]: trait.MmapFileMutExt.html#methods.flush_async
/// [`flush_async_range`]: trait.MmapFileMutExt.html#methods.flush_async_range
/// [`MmapFileMutExt`]: trait.MmapFileMutExt.html
pub struct MmapFileWriter<'a> {
    w: io::Cursor<&'a mut [u8]>,
    offset: usize,
    len: usize,
}

impl<'a> MmapFileWriter<'a> {
    pub(crate) fn new(w: io::Cursor<&'a mut [u8]>, offset: usize, len: usize) -> Self {
        Self {
            w,
            offset,
            len
        }
    }

    /// Returns the start offset(related to the mmap) of the writer
    #[inline]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the length of the writer
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
}

impl Debug for MmapFileWriter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MmapFileWriter")
            .field("offset", &self.offset)
            .field("len", &self.len)
            .field("writer", &self.w)
            .finish()
    }
}

impl io::Read for MmapFileWriter<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.w.read(buf)
    }
}

impl io::BufRead for MmapFileWriter<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.w.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.w.consume(amt)
    }
}

impl io::Write for MmapFileWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.w.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

impl io::Seek for MmapFileWriter<'_> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.w.seek(pos)
    }
}

impl Buf for MmapFileWriter<'_> {
    fn remaining(&self) -> usize {
        self.w.remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.w.chunk()
    }

    fn advance(&mut self, cnt: usize) {
        self.w.advance(cnt)
    }
}

/// Extends MmapFileWriter with methods for writing numbers.
pub trait MmapFileWriterExt {
    /// Writes a signed 8 bit integer to the underlying writer.
    /// Note that since this writes a single byte, no byte order conversions are used. It is included for completeness.
    fn write_i8(&mut self, n: i8) -> io::Result<()>;
    /// Writes a signed 16 bit integer(big endian) to the underlying writer.
    fn write_i16(&mut self, n: i16) -> io::Result<()>;
    /// Writes a signed 16 bit integer(little endian) to the underlying writer.
    fn write_i16_le(&mut self, n: i16) -> io::Result<()>;
    /// Writes a signed 32 bit integer(big endian) to the underlying writer.
    fn write_i32(&mut self, n: i32) -> io::Result<()>;
    /// Writes a signed 32 bit integer(little endian) to the underlying writer.
    fn write_i32_le(&mut self, n: i32) -> io::Result<()>;
    /// Writes a signed 64 bit integer(big endian) to the underlying writer.
    fn write_i64(&mut self, n: i64) -> io::Result<()>;
    /// Writes a signed 64 bit integer(little endian) to the underlying writer.
    fn write_i64_le(&mut self, n: i64) -> io::Result<()>;
    /// Writes a signed integer(big endian) to the underlying writer.
    fn write_isize(&mut self, n: isize) -> io::Result<()>;
    /// Writes a signed integer(little endian) to the underlying writer.
    fn write_isize_le(&mut self, n: isize) -> io::Result<()>;
    /// Writes a signed 128 bit integer(big endian) to the underlying writer.
    fn write_i128(&mut self, n: i128) -> io::Result<()>;
    /// Writes a signed 128 bit integer(little endian) to the underlying writer.
    fn write_i128_le(&mut self, n: i128) -> io::Result<()>;

    /// Writes an unsigned 8 bit integer to the underlying writer.
    /// Note that since this writes a single byte, no byte order conversions are used. It is included for completeness.
    fn write_u8(&mut self, n: u8) -> io::Result<()>;
    /// Writes an unsigned 16 bit integer(big endian) to the underlying writer.
    fn write_u16(&mut self, n: u16) -> io::Result<()>;
    /// Writes an unsigned 16 bit integer(little endian) to the underlying writer.
    fn write_u16_le(&mut self, n: u16) -> io::Result<()>;
    /// Writes an unsigned 32 bit integer(big endian) to the underlying writer.
    fn write_u32(&mut self, n: u32) -> io::Result<()>;
    /// Writes an unsigned 32 bit integer(little endian) to the underlying writer.
    fn write_u32_le(&mut self, n: u32) -> io::Result<()>;
    /// Writes an unsigned 64 bit integer(big endian) to the underlying writer.
    fn write_u64(&mut self, n: u64) -> io::Result<()>;
    /// Writes an unsigned 64 bit integer(little endian) to the underlying writer.
    fn write_u64_le(&mut self, n: u64) -> io::Result<()>;
    /// Writes an unsigned integer(big endian) to the underlying writer.
    fn write_usize(&mut self, n: usize) -> io::Result<()>;
    /// Writes an unsigned integer(little endian) to the underlying writer.
    fn write_usize_le(&mut self, n: usize) -> io::Result<()>;
    /// Writes an unsigned 128 bit integer(big endian) to the underlying writer.
    fn write_u128(&mut self, n: u128) -> io::Result<()>;
    /// Writes an unsigned 128 bit integer(little endian) to the underlying writer.
    fn write_u128_le(&mut self, n: u128) -> io::Result<()>;

    /// Writes a IEEE754 single-precision (4 bytes, big endian) floating point number to the underlying writer.
    fn write_f32(&mut self, n: f32) -> io::Result<()>;
    /// Writes a IEEE754 single-precision (4 bytes, little endian) floating point number to the underlying writer.
    fn write_f32_le(&mut self, n: f32) -> io::Result<()>;
    /// Writes a IEEE754 single-precision (8 bytes, big endian) floating point number to the underlying writer
    fn write_f64(&mut self, n: f64) -> io::Result<()>;
    /// Writes a IEEE754 single-precision (8 bytes, little endian) floating point number to the underlying writer
    fn write_f64_le(&mut self, n: f64) -> io::Result<()>;
}

impl MmapFileWriterExt for MmapFileWriter<'_> {
    #[inline]
    fn write_i8(&mut self, n: i8) -> io::Result<()> {
        self.w.write_i8(n)
    }

    #[inline]
    fn write_i16(&mut self, n: i16) -> io::Result<()> {
        self.w.write_i16::<BigEndian>(n)
    }

    #[inline]
    fn write_i16_le(&mut self, n: i16) -> io::Result<()> {
        self.w.write_i16::<LittleEndian>(n)
    }

    #[inline]
    fn write_i32(&mut self, n: i32) -> io::Result<()> {
        self.w.write_i32::<BigEndian>(n)
    }

    #[inline]
    fn write_i32_le(&mut self, n: i32) -> io::Result<()> {
        self.w.write_i32::<LittleEndian>(n)
    }

    #[inline]
    fn write_i64(&mut self, n: i64) -> io::Result<()> {
        self.w.write_i64::<BigEndian>(n)
    }

    #[inline]
    fn write_i64_le(&mut self, n: i64) -> io::Result<()> {
        self.w.write_i64::<LittleEndian>(n)
    }

    #[inline]
    fn write_isize(&mut self, n: isize) -> io::Result<()> {
        self.w.write_all(n.to_be_bytes().as_ref())
    }

    #[inline]
    fn write_isize_le(&mut self, n: isize) -> io::Result<()> {
        self.w.write_all(n.to_le_bytes().as_ref())
    }

    #[inline]
    fn write_i128(&mut self, n: i128) -> io::Result<()> {
        self.w.write_all(n.to_be_bytes().as_ref())
    }

    #[inline]
    fn write_i128_le(&mut self, n: i128) -> io::Result<()> {
        self.w.write_all(n.to_le_bytes().as_ref())
    }

    #[inline]
    fn write_u8(&mut self, n: u8) -> io::Result<()> {
        self.w.write_u8(n)
    }

    #[inline]
    fn write_u16(&mut self, n: u16) -> io::Result<()> {
        self.w.write_u16::<BigEndian>(n)
    }

    #[inline]
    fn write_u16_le(&mut self, n: u16) -> io::Result<()> {
        self.w.write_u16::<LittleEndian>(n)
    }

    #[inline]
    fn write_u32(&mut self, n: u32) -> io::Result<()> {
        self.w.write_u32::<BigEndian>(n)
    }

    #[inline]
    fn write_u32_le(&mut self, n: u32) -> io::Result<()> {
        self.w.write_u32::<LittleEndian>(n)
    }

    #[inline]
    fn write_u64(&mut self, n: u64) -> io::Result<()> {
        self.w.write_u64::<BigEndian>(n)
    }

    #[inline]
    fn write_u64_le(&mut self, n: u64) -> io::Result<()> {
        self.w.write_u64::<LittleEndian>(n)
    }

    #[inline]
    fn write_usize(&mut self, n: usize) -> io::Result<()> {
        self.w.write_all(n.to_be_bytes().as_ref())
    }

    #[inline]
    fn write_usize_le(&mut self, n: usize) -> io::Result<()> {
        self.w.write_all(n.to_le_bytes().as_ref())
    }

    #[inline]
    fn write_u128(&mut self, n: u128) -> io::Result<()> {
        self.w.write_all(n.to_be_bytes().as_ref())
    }

    #[inline]
    fn write_u128_le(&mut self, n: u128) -> io::Result<()> {
        self.w.write_all(n.to_le_bytes().as_ref())
    }

    #[inline]
    fn write_f32(&mut self, n: f32) -> io::Result<()> {
        self.w.write_f32::<BigEndian>(n)
    }

    #[inline]
    fn write_f32_le(&mut self, n: f32) -> io::Result<()> {
        self.w.write_f32::<LittleEndian>(n)
    }

    #[inline]
    fn write_f64(&mut self, n: f64) -> io::Result<()> {
        self.w.write_f64::<BigEndian>(n)
    }

    #[inline]
    fn write_f64_le(&mut self, n: f64) -> io::Result<()> {
        self.w.write_f64::<LittleEndian>(n)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, Read};
    use bytes::Buf;
    use crate::MmapFileMutExt;
    use crate::raw::MemoryMmapFileMut;

    #[test]
    fn test_writer() {
        let mut file = MemoryMmapFileMut::from_vec("test.mem", vec![1; 8096]);
        let mut w = file.writer(0).unwrap();
        let _ = format!("{:?}", w);
        assert_eq!(w.len(), 8096);
        assert_eq!(w.offset(), 0);
        let mut buf = [0; 10];
        let n = w.read(&mut buf).unwrap();
        assert!(buf[0..n].eq(vec![1; n].as_slice()));
        w.fill_buf().unwrap();
        w.consume(8096);

        let mut w = file.range_writer(100, 100).unwrap();
        assert_eq!(w.remaining(), 100);
        w.advance(10);
        assert_eq!(w.remaining(), 90);
        let buf = w.chunk();
        assert_eq!(buf.len(), 90);
    }
}