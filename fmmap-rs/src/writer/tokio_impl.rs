use std::fmt::{Debug, Formatter};
use std::io::{Error, SeekFrom, Cursor};
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::Buf;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};
use pin_project_lite::pin_project;

declare_and_impl_basic_writer!();

impl AsyncRead for AsyncMmapFileWriter<'_> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        self.project().w.poll_read(cx, buf)
    }
}

impl AsyncBufRead for AsyncMmapFileWriter<'_> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        self.project().w.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().w.consume(amt)
    }
}

impl AsyncSeek for AsyncMmapFileWriter<'_> {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        self.project().w.start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        self.project().w.poll_complete(cx)
    }
}

impl AsyncWrite for AsyncMmapFileWriter<'_> {
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

impl Buf for AsyncMmapFileWriter<'_> {
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

#[cfg(test)]
mod tests {
    use bytes::Buf;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
    use crate::tokio::AsyncMmapFileMutExt;
    use crate::raw::tokio::AsyncMemoryMmapFileMut;

    #[tokio::test]
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