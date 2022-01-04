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

#[cfg(test)]
mod tests {
    use futures_util::{AsyncBufReadExt, AsyncReadExt};
    use crate::async_std::AsyncMmapFileExt;
    use crate::raw::async_std::AsyncMemoryMmapFileMut;

    #[async_std::test]
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
    }
}