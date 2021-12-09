use std::fmt::{Debug, Formatter};
use std::io::{Error, SeekFrom, Cursor};
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::BytesMut;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use pin_project::pin_project;

/// AsyncMmapFileWriter helps read or write data from mmap file
/// like a normal file.
///
/// # Notes
/// If you use a writer to write data to mmap, there is no guarantee all
/// data will be durably stored. So you need to call [`flush`]/[`flush_range`]/[`flush_async`]/[`flush_async_range`] in [`AsyncMmapFileMutExt`]
/// to guarantee all data will be durably stored.
///
/// [`flush`]: traits.AsyncMmapFileMutExt.html#methods.flush
/// [`flush_range`]: traits.AsyncMmapFileMutExt.html#methods.flush_range
/// [`flush_async`]: traits.AsyncMmapFileMutExt.html#methods.flush_async
/// [`flush_async_range`]: traits.AsyncMmapFileMutExt.html#methods.flush_async_range
/// [`AsyncMmapFileMutExt`]: traits.AsyncMmapFileMutExt.html
#[pin_project]
pub struct AsyncMmapFileWriter<'a> {
    #[pin]
    w: Cursor<&'a mut [u8]>,
    offset: usize,
    len: usize,
}

impl<'a> AsyncMmapFileWriter<'a> {
    pub(crate) fn new(w: Cursor<&'a mut [u8]>, offset: usize, len: usize) -> Self {
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

impl Debug for AsyncMmapFileWriter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncMmapFileWriter")
            .field("offset", &self.offset)
            .field("len", &self.len)
            .field("writer", &self.w)
            .finish()
    }
}

impl<'a> AsyncRead for AsyncMmapFileWriter<'a> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        self.project().w.poll_read(cx, buf)
    }
}

impl<'a> AsyncReadExt for AsyncMmapFileWriter<'a>  {}

impl<'a> AsyncBufRead for AsyncMmapFileWriter<'a> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        self.project().w.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().w.consume(amt)
    }
}

impl<'a> AsyncBufReadExt for AsyncMmapFileWriter<'a> {}

impl<'a> AsyncSeek for AsyncMmapFileWriter<'a> {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        self.project().w.start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        self.project().w.poll_complete(cx)
    }
}

impl<'a> AsyncSeekExt for AsyncMmapFileWriter<'a> {}

impl<'a> AsyncWrite for AsyncMmapFileWriter<'a> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        self.project().w.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.project().w.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.project().w.poll_shutdown()

    }
}

impl<'a> AsyncWriteExt for AsyncMmapFileWriter<'a> {}
