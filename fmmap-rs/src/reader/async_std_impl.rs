use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};
use pin_project_lite::pin_project;
use async_std::io::{Cursor, SeekFrom, Read, BufRead, Seek};

declare_and_impl_basic_reader!();

impl<'a> Read for AsyncMmapFileReader<'a> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        self.project().r.poll_read(cx, buf)
    }
}

impl<'a> BufRead for AsyncMmapFileReader<'a> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        self.project().r.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().r.consume(amt)
    }
}

impl<'a> Seek for AsyncMmapFileReader<'a> {
    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<std::io::Result<u64>> {
        self.project().r.poll_seek(cx, pos)
    }
}