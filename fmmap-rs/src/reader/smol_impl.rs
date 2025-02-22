use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};
use pin_project_lite::pin_project;
use smol::io::{AsyncBufRead, AsyncRead, AsyncSeek, Cursor, SeekFrom};

declare_and_impl_basic_reader!();

impl AsyncRead for AsyncMmapFileReader<'_> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        self.project().r.poll_read(cx, buf)
    }
}

impl AsyncSeek for AsyncMmapFileReader<'_> {
    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<std::io::Result<u64>> {
        self.project().r.poll_seek(cx, pos)
    }
}

impl AsyncBufRead for AsyncMmapFileReader<'_> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        self.project().r.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().r.consume(amt)
    }
}

#[cfg(test)]
mod tests {
    use smol::io::{AsyncBufReadExt, AsyncReadExt};
    use crate::smol::AsyncMmapFileExt;
    use crate::raw::smol::AsyncMemoryMmapFileMut;

    #[smol_potat::test]
    async fn test_reader() {
        let file = AsyncMemoryMmapFileMut::from_vec("test.mem", vec![1; 8096]);
        let mut w = file.reader(0).unwrap();
        let _ = format!("{:?}", w);
        assert_eq!(w.len(), 8096);
        assert_eq!(w.offset(), 0);
        let mut buf = [0; 10];
        let n = w.read(&mut buf).await.unwrap();
        assert!(buf[0..n].eq(vec![1; n].as_slice()));
        w.fill_buf().await.unwrap();
        w.consume(8096);
    }
}