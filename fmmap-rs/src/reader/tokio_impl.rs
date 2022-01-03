use bytes::Buf;
use std::fmt::{Debug, Formatter};
use std::io::{Cursor, SeekFrom};
use std::pin::Pin;
use std::task::{Context, Poll};
use pin_project_lite::pin_project;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncSeek, ReadBuf};

declare_and_impl_basic_reader!();

impl<'a> AsyncRead for AsyncMmapFileReader<'a> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        self.project().r.poll_read(cx, buf)
    }
}

impl<'a> AsyncSeek for AsyncMmapFileReader<'a> {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        self.project().r.start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        self.project().r.poll_complete(cx)
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

impl<'a> Buf for AsyncMmapFileReader<'a> {
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

#[cfg(test)]
mod tests {
    use bytes::Buf;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
    use crate::tokio::{AsyncMmapFileExt, AsyncMmapFileMutExt};
    use crate::raw::tokio::AsyncMemoryMmapFileMut;

    #[tokio::test]
    async fn test_reader() {
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

        let mut w = file.range_reader(100, 100).unwrap();
        assert_eq!(w.remaining(), 100);
        w.advance(10);
        assert_eq!(w.remaining(), 90);
        let buf = w.chunk();
        assert_eq!(buf.len(), 90);
    }
}