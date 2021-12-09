use std::fmt::{Debug, Formatter};
use std::io::{Cursor, SeekFrom};
use std::pin::Pin;
use std::task::{Context, Poll};
use pin_project::pin_project;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, ReadBuf};

/// AsyncMmapFileReader helps read data from mmap file
/// like a normal file.
#[pin_project]
pub struct AsyncMmapFileReader<'a> {
    #[pin]
    r: Cursor<&'a [u8]>,
    offset: usize,
    len: usize,
}

impl<'a> AsyncMmapFileReader<'a> {
    pub(crate) fn new(r: Cursor<&'a [u8]>, offset: usize, len: usize) -> Self {
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

impl Debug for AsyncMmapFileReader<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncMmapFileReader")
            .field("offset", &self.offset)
            .field("len", &self.len)
            .field("reader", &self.r)
            .finish()
    }
}

impl<'a> AsyncRead for AsyncMmapFileReader<'a> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        self.r.poll_read(cx, buf)
    }
}

impl<'a> AsyncReadExt for AsyncMmapFileReader<'a> {}

impl<'a> AsyncSeek for AsyncMmapFileReader<'a> {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        self.project().r.start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        self.project().r.poll_complete(cx)
    }
}

impl<'a> AsyncSeekExt for AsyncMmapFileReader<'a> {}

impl<'a> AsyncBufRead for AsyncMmapFileReader<'a> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        self.project().r.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().r.consume(amt)
    }
}

impl<'a> AsyncBufReadExt for AsyncMmapFileReader<'a> {}