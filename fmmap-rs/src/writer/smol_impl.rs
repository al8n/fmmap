use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};
use smol::io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite, Cursor, SeekFrom};
use pin_project_lite::pin_project;

declare_and_impl_basic_writer!();

impl AsyncRead for AsyncMmapFileWriter<'_> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
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
    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<std::io::Result<u64>> {
        self.project().w.poll_seek(cx, pos)
    }
}

impl AsyncWrite for AsyncMmapFileWriter<'_> {
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
    use smol::io::{AsyncBufReadExt, AsyncWriteExt, AsyncReadExt};
    use crate::smol::AsyncMmapFileMutExt;
    use crate::raw::smol::AsyncMemoryMmapFileMut;

    #[smol_potat::test]
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
        w.close().await.unwrap();
    }
}