use std::fmt::{Debug, Formatter};
use std::io;
use std::io::Read;
use std::mem;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::Buf;

/// MmapFileReader helps read data from mmap file
/// like a normal file.
pub struct MmapFileReader<'a> {
    r: io::Cursor<&'a [u8]>,
    offset: usize,
    len: usize,
}

impl<'a> MmapFileReader<'a> {
    pub(crate) fn new(r: io::Cursor<&'a [u8]>, offset: usize, len: usize) -> Self {
        Self {
            r,
            offset,
            len
        }
    }

    /// Returns the start offset(related to the mmap) of the reader
    #[inline]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the length of the reader
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
}


impl Debug for MmapFileReader<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MmapFileReader")
            .field("offset", &self.offset)
            .field("len", &self.len)
            .field("reader", &self.r)
            .finish()
    }
}

impl<'a> io::Seek for MmapFileReader<'a> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.r.seek(pos)
    }
}

impl<'a> io::BufRead for MmapFileReader<'a> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.r.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.r.consume(amt)
    }
}

impl<'a> io::Read for MmapFileReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.r.read(buf)
    }
}

impl<'a> Buf for MmapFileReader<'a> {
    fn remaining(&self) -> usize {
        self.r.remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.r.chunk()
    }

    fn advance(&mut self, cnt: usize) {
        self.r.advance(cnt)
    }
}

/// Extends MmapFileReader with methods for reading numbers.
pub trait MmapFileReaderExt {
    /// Reads a signed 8 bit integer from the underlying reader.
    /// Note that since this reads a single byte, no byte order conversions are used. It is included for completeness.
    fn read_i8(&mut self) -> io::Result<i8>;
    /// Reads a signed 16 bit integer(big endian) from the underlying reader.
    fn read_i16(&mut self) -> io::Result<i16>;
    /// Reads a signed 16 bit integer(little endian) from the underlying reader.
    fn read_i16_le(&mut self) -> io::Result<i16>;
    /// Reads a signed 16 bit integer(big endian) from the underlying reader.
    fn read_i32(&mut self) -> io::Result<i32>;
    /// Reads a signed 32 bit integer(little endian) from the underlying reader.
    fn read_i32_le(&mut self) -> io::Result<i32>;
    /// Reads a signed 64 bit integer(big endian) from the underlying reader.
    fn read_i64(&mut self) -> io::Result<i64>;
    /// Reads a signed 64 bit integer(little endian) from the underlying reader.
    fn read_i64_le(&mut self) -> io::Result<i64>;
    /// Reads a signed integer(big endian) from the underlying reader.
    fn read_isize(&mut self) -> io::Result<isize>;
    /// Reads a signed integer(little endian) from the underlying reader.
    fn read_isize_le(&mut self) -> io::Result<isize>;
    /// Reads a signed 128 bit integer(big endian) from the underlying reader.
    fn read_i128(&mut self) -> io::Result<i128>;
    /// Reads a signed 128 bit integer(little endian) from the underlying reader.
    fn read_i128_le(&mut self) -> io::Result<i128>;

    /// Reads an unsigned 8 bit integer from the underlying reader.
    /// Note that since this reads a single byte, no byte order conversions are used. It is included for completeness.
    fn read_u8(&mut self) -> io::Result<u8>;
    /// Reads an unsigned 16 bit integer(big endian) from the underlying reader.
    fn read_u16(&mut self) -> io::Result<u16>;
    /// Reads an unsigned 16 bit integer(little endian) from the underlying reader.
    fn read_u16_le(&mut self) -> io::Result<u16>;
    /// Reads an unsigned 32 bit integer(big endian) from the underlying reader.
    fn read_u32(&mut self) -> io::Result<u32>;
    /// Reads an unsigned 32 bit integer(little endian) from the underlying reader.
    fn read_u32_le(&mut self) -> io::Result<u32>;
    /// Reads an unsigned 64 bit integer(big endian) from the underlying reader.
    fn read_u64(&mut self) -> io::Result<u64>;
    /// Reads an unsigned 64 bit integer(little endian) from the underlying reader.
    fn read_u64_le(&mut self) -> io::Result<u64>;
    /// Reads an unsigned integer(big endian) from the underlying reader.
    fn read_usize(&mut self) -> io::Result<usize>;
    /// Reads an unsigned integer(little endian) from the underlying reader.
    fn read_usize_le(&mut self) -> io::Result<usize>;
    /// Reads an unsigned 128 bit integer(big endian) from the underlying reader.
    fn read_u128(&mut self) -> io::Result<u128>;
    /// Reads an unsigned 128 bit integer(little endian) from the underlying reader.
    fn read_u128_le(&mut self) -> io::Result<u128>;

    /// Reads a IEEE754 single-precision (4 bytes, big endian) floating point number from the underlying reader.
    fn read_f32(&mut self) -> io::Result<f32>;
    /// Reads a IEEE754 single-precision (4 bytes, little endian) floating point number from the underlying reader.
    fn read_f32_le(&mut self) -> io::Result<f32>;
    /// Reads a IEEE754 single-precision (8 bytes, big endian) floating point number from the underlying reader.
    fn read_f64(&mut self) -> io::Result<f64>;
    /// Reads a IEEE754 single-precision (8 bytes, little endian) floating point number from the underlying reader.
    fn read_f64_le(&mut self) -> io::Result<f64>;
}

impl<'a> MmapFileReaderExt for MmapFileReader<'a> {
    fn read_i8(&mut self) -> io::Result<i8> {
        self.r.read_i8()
    }

    fn read_i16(&mut self) -> io::Result<i16> {
        self.r.read_i16::<BigEndian>()
    }

    fn read_i16_le(&mut self) -> io::Result<i16> {
        self.r.read_i16::<LittleEndian>()
    }

    fn read_i32(&mut self) -> io::Result<i32> {
        self.r.read_i32::<BigEndian>()
    }

    fn read_i32_le(&mut self) -> io::Result<i32> {
        self.r.read_i32::<LittleEndian>()
    }

    fn read_i64(&mut self) -> io::Result<i64> {
        self.r.read_i64::<BigEndian>()
    }

    fn read_i64_le(&mut self) -> io::Result<i64> {
        self.r.read_i64::<LittleEndian>()
    }

    fn read_isize(&mut self) -> io::Result<isize> {
        const ISIZE_SIZE: usize = mem::size_of::<isize>();
        let mut buf: [u8; ISIZE_SIZE] = [0; ISIZE_SIZE];
        self.r.read_exact(&mut buf)?;
        Ok(isize::from_be_bytes(buf))
    }

    fn read_isize_le(&mut self) -> io::Result<isize> {
        const ISIZE_SIZE: usize = mem::size_of::<isize>();
        let mut buf: [u8; ISIZE_SIZE] = [0; ISIZE_SIZE];
        self.r.read_exact(&mut buf)?;
        Ok(isize::from_le_bytes(buf))
    }

    fn read_i128(&mut self) -> io::Result<i128> {
        const I128_SIZE: usize = mem::size_of::<i128>();
        let mut buf: [u8; I128_SIZE] = [0; I128_SIZE];
        self.r.read_exact(&mut buf)?;
        Ok(i128::from_be_bytes(buf))
    }

    fn read_i128_le(&mut self) -> io::Result<i128> {
        const I128_SIZE: usize = mem::size_of::<i128>();
        let mut buf: [u8; I128_SIZE] = [0; I128_SIZE];
        self.r.read_exact(&mut buf)?;
        Ok(i128::from_le_bytes(buf))
    }

    fn read_u8(&mut self) -> io::Result<u8> {
        self.r.read_u8()
    }

    fn read_u16(&mut self) -> io::Result<u16> {
        self.r.read_u16::<BigEndian>()
    }

    fn read_u16_le(&mut self) -> io::Result<u16> {
        self.r.read_u16::<LittleEndian>()
    }

    fn read_u32(&mut self) -> io::Result<u32> {
        self.r.read_u32::<BigEndian>()
    }

    fn read_u32_le(&mut self) -> io::Result<u32> {
        self.r.read_u32::<LittleEndian>()
    }

    fn read_u64(&mut self) -> io::Result<u64> {
        self.r.read_u64::<BigEndian>()
    }

    fn read_u64_le(&mut self) -> io::Result<u64> {
        self.r.read_u64::<LittleEndian>()
    }

    fn read_usize(&mut self) -> io::Result<usize> {
        const USIZE_SIZE: usize = mem::size_of::<usize>();
        let mut buf: [u8; USIZE_SIZE] = [0; USIZE_SIZE];
        self.r.read_exact(&mut buf)?;
        Ok(usize::from_le_bytes(buf))
    }

    fn read_usize_le(&mut self) -> io::Result<usize> {
        const USIZE_SIZE: usize = mem::size_of::<usize>();
        let mut buf: [u8; USIZE_SIZE] = [0; USIZE_SIZE];
        self.r.read_exact(&mut buf)?;
        Ok(usize::from_le_bytes(buf))
    }

    fn read_u128(&mut self) -> io::Result<u128> {
        const U128_SIZE: usize = mem::size_of::<u128>();
        let mut buf: [u8; U128_SIZE] = [0; U128_SIZE];
        self.r.read_exact(&mut buf)?;
        Ok(u128::from_be_bytes(buf))
    }

    fn read_u128_le(&mut self) -> io::Result<u128> {
        const U128_SIZE: usize = mem::size_of::<u128>();
        let mut buf: [u8; U128_SIZE] = [0; U128_SIZE];
        self.r.read_exact(&mut buf)?;
        Ok(u128::from_le_bytes(buf))
    }

    fn read_f32(&mut self) -> io::Result<f32> {
        self.r.read_f32::<BigEndian>()
    }

    fn read_f32_le(&mut self) -> io::Result<f32> {
        self.r.read_f32::<LittleEndian>()
    }

    fn read_f64(&mut self) -> io::Result<f64> {
        self.r.read_f64::<BigEndian>()
    }

    fn read_f64_le(&mut self) -> io::Result<f64> {
        self.r.read_f64::<LittleEndian>()
    }
}