use std::fmt::{Debug, Formatter};
use std::io;
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