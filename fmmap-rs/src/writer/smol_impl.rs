use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};
use smol::io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite, Cursor, SeekFrom};
use pin_project_lite::pin_project;

declare_and_impl_basic_writer!();

impl<'a> AsyncRead for AsyncMmapFileWriter<'a> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
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

impl<'a> AsyncSeek for AsyncMmapFileWriter<'a> {
    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<std::io::Result<u64>> {
        self.project().w.poll_seek(cx, pos)
    }
}

impl<'a> AsyncWrite for AsyncMmapFileWriter<'a> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        self.project().w.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.project().w.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.project().w.poll_close(cx)
    }
}
