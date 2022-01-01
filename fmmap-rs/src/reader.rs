cfg_sync!(
    mod sync_impl;
    pub use sync_impl::{MmapFileReader, MmapFileReaderExt};
);

cfg_async! {
    macro_rules! declare_and_impl_basic_reader {
        () => {
            pin_project! {
                /// AsyncMmapFileReader helps read data from mmap file
                /// like a normal file.
                pub struct AsyncMmapFileReader<'a> {
                    #[pin]
                    r: Cursor<&'a [u8]>,
                    offset: usize,
                    len: usize,
                }
            }


            impl<'a> AsyncMmapFileReader<'a> {
                pub(crate) fn new(r: Cursor<&'a [u8]>, offset: usize, len: usize) -> Self {
                    Self {
                        r,
                        offset,
                        len
                    }
                }

                /// Returns the start offset(related to the mmap) of the reader
                #[inline]
                pub fn offset(&self) -> usize {
                    self.offset
                }

                /// Returns the length of the reader
                #[inline]
                pub fn len(&self) -> usize {
                    self.len
                }
            }

            impl Debug for AsyncMmapFileReader<'_> {
                fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct("AsyncMmapFileReader")
                        .field("offset", &self.offset)
                        .field("len", &self.len)
                        .field("reader", &self.r)
                        .finish()
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
        };
    }
}

cfg_async_std!(
    pub(crate) mod async_std_impl;
);

cfg_smol!(
    pub(crate) mod smol_impl;
);

cfg_tokio!(
    pub(crate) mod tokio_impl;
);

