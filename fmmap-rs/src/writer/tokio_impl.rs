use std::fmt::{Debug, Formatter};
use std::io::{Error, SeekFrom, Cursor};
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::Buf;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};
use pin_project::pin_project;

/// AsyncMmapFileWriter helps read or write data from mmap file
/// like a normal file.
///
/// # Notes
/// If you use a writer to write data to mmap, there is no guarantee all
/// data will be durably stored. So you need to call [`flush`]/[`flush_range`]/[`flush_async`]/[`flush_async_range`] in [`AsyncMmapFileMutExt`]
/// to guarantee all data will be durably stored.
///
/// [`flush`]: trait.AsyncMmapFileMutExt.html#methods.flush
/// [`flush_range`]: trait.AsyncMmapFileMutExt.html#methods.flush_range
/// [`flush_async`]: trait.AsyncMmapFileMutExt.html#methods.flush_async
/// [`flush_async_range`]: trait.AsyncMmapFileMutExt.html#methods.flush_async_range
/// [`AsyncMmapFileMutExt`]: trait.AsyncMmapFileMutExt.html
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
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        self.project().w.poll_read(cx, buf)
    }
}

impl<'a> AsyncBufRead for AsyncMmapFileWriter<'a> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        self.project().w.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().w.consume(amt)
    }
}

impl<'a> Buf for AsyncMmapFileWriter<'a> {
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

impl<'a> AsyncSeek for AsyncMmapFileWriter<'a> {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        self.project().w.start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        self.project().w.poll_complete(cx)
    }
}

impl<'a> AsyncWrite for AsyncMmapFileWriter<'a> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        self.project().w.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.project().w.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.project().w.poll_shutdown(cx)

    }
}

#[cfg(test)]
mod tests {
    use bytes::Buf;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
    use crate::{AsyncMmapFileMutExt};
    use crate::raw::AsyncMemoryMmapFileMut;

    #[tokio::test]
    async fn test_writer() {
        let mut file = AsyncMemoryMmapFileMut::from_vec("test.mem", vec![1; 8096]);
        let mut w = file.writer(0).unwrap();
        assert_eq!(w.len(), 8096);
        assert_eq!(w.offset(), 0);
        let mut buf = [0; 10];
        let n = w.read(&mut buf).await.unwrap();
        assert!(buf[0..n].eq(vec![1; n].as_slice()));
        w.fill_buf().await.unwrap();
        w.consume(8096);
        w.shutdown().await.unwrap();

        let mut w = file.range_writer(100, 100).unwrap();
        assert_eq!(w.remaining(), 100);
        w.advance(10);
        assert_eq!(w.remaining(), 90);
        let buf = w.chunk();
        assert_eq!(buf.len(), 90);
    }
}