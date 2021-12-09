use std::fmt::{Debug, Formatter};
use std::io;
use bytes::Buf;


/// MmapFileWriter helps read or write data from mmap file
/// like a normal file.
///
/// # Notes
/// If you use a writer to write data to mmap, there is no guarantee all
/// data will be durably stored. So you need to call [`flush`]/[`flush_range`]/[`flush_async`]/[`flush_async_range`] in [`MmapFileMutExt`]
/// to guarantee all data will be durably stored.
///
/// [`flush`]: traits.MmapFileMutExt.html#methods.flush
/// [`flush_range`]: traits.MmapFileMutExt.html#methods.flush_range
/// [`flush_async`]: traits.MmapFileMutExt.html#methods.flush_async
/// [`flush_async_range`]: traits.MmapFileMutExt.html#methods.flush_async_range
/// [`MmapFileMutExt`]: traits.MmapFileMutExt.html
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

impl<'a> io::Read for MmapFileWriter<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.w.read(buf)
    }
}

impl<'a> io::BufRead for MmapFileWriter<'a> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.w.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.w.consume(amt)
    }
}

impl<'a> io::Write for MmapFileWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.w.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

impl<'a> io::Seek for MmapFileWriter<'a> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.w.seek(pos)
    }
}

impl<'a> Buf for MmapFileWriter<'a> {
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
