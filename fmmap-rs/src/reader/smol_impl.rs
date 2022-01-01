use bytes::Buf;
use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};
use pin_project_lite::pin_project;
use smol::io::{AsyncBufRead, AsyncRead, AsyncSeek, Cursor, SeekFrom};

declare_and_impl_basic_reader!();

impl<'a> AsyncRead for AsyncMmapFileReader<'a> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        self.project().r.poll_read(cx, buf)
    }
}

impl<'a> AsyncSeek for AsyncMmapFileReader<'a> {
    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<std::io::Result<u64>> {
        self.project().r.poll_seek(cx, pos)
    }
}

impl<'a> AsyncBufRead for AsyncMmapFileReader<'a> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        self.project().r.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().r.consume(amt)
    }
}