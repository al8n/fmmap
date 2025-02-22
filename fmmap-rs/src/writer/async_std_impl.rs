use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};
use async_std::io::{Cursor, Read, BufRead, Seek, SeekFrom, Write};
use pin_project_lite::pin_project;

declare_and_impl_basic_writer!();

impl Read for AsyncMmapFileWriter<'_> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        self.project().w.poll_read(cx, buf)
    }
}

impl BufRead for AsyncMmapFileWriter<'_> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        self.project().w.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().w.consume(amt)
    }
}

impl Seek for AsyncMmapFileWriter<'_> {
    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<std::io::Result<u64>> {
        self.project().w.poll_seek(cx, pos)
    }
}

impl Write for AsyncMmapFileWriter<'_> {
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

#[cfg(test)]
mod tests {
    use futures_util::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
    use crate::async_std::AsyncMmapFileMutExt;
    use crate::raw::async_std::AsyncMemoryMmapFileMut;

    #[async_std::test]
    async fn test_writer() {
        let mut file = AsyncMemoryMmapFileMut::from_vec("test.mem", vec![1; 8096]);
        let mut w = file.writer(0).unwrap();
        let _ = format!("{:?}", w);
        assert_eq!(w.len(), 8096);
        assert_eq!(w.offset(), 0);
        let mut buf = [0; 10];
        let n = w.read(&mut buf).await.unwrap();
        assert!(buf[0..n].eq(vec![1; n].as_slice()));
        w.fill_buf().await.unwrap();
        w.close().await.unwrap();
    }
}

