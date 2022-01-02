cfg_sync!(
    mod sync_impl;
    pub use sync_impl::{MmapFileWriter, MmapFileWriterExt};
);

cfg_async! {
    macro_rules! declare_and_impl_basic_writer {
        () => {
            pin_project! {
                /// AsyncMmapFileWriter helps read or write data from mmap file
                /// like a normal file.
                ///
                /// # Notes
                /// If you use a writer to write data to mmap, there is no guarantee all
                /// data will be durably stored. So you need to call [`flush`]/[`flush_range`]/[`flush_async`]/[`flush_async_range`] in [`AsyncMmapFileMutExt`]
                /// to guarantee all data will be durably stored.
                ///
                /// [`flush`]: trait.AsyncMmapFileMutExt.html#methods.flush
                /// [`flush_range`]: trait.AsyncMmapFileMutExt.html#methods.flush_range
                /// [`flush_async`]: trait.AsyncMmapFileMutExt.html#methods.flush_async
                /// [`flush_async_range`]: trait.AsyncMmapFileMutExt.html#methods.flush_async_range
                /// [`AsyncMmapFileMutExt`]: trait.AsyncMmapFileMutExt.html
                pub struct AsyncMmapFileWriter<'a> {
                    #[pin]
                    w: Cursor<&'a mut [u8]>,
                    offset: usize,
                    len: usize,
                }
            }

            impl<'a> AsyncMmapFileWriter<'a> {
                pub(crate) fn new(w: Cursor<&'a mut [u8]>, offset: usize, len: usize) -> Self {
                    Self {
                        w,
                        offset,
                        len
                    }
                }

                /// Returns the start offset(related to the mmap) of the writer
                #[inline]
                pub fn offset(&self) -> usize {
                    self.offset
                }

                /// Returns the length of the writer
                #[inline]
                pub fn len(&self) -> usize {
                    self.len
                }
            }

            impl Debug for AsyncMmapFileWriter<'_> {
                fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct("AsyncMmapFileWriter")
                        .field("offset", &self.offset)
                        .field("len", &self.len)
                        .field("writer", &self.w)
                        .finish()
                }
            }
        };
    }
}

cfg_async_std!(pub mod async_std_impl;);
cfg_smol!(pub mod smol_impl;);
cfg_tokio!(pub mod tokio_impl;);

