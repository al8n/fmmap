#[cfg(feature = "sync")]
#[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
mod sync_impl;
#[cfg(feature = "sync")]
#[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
pub use sync_impl::{MmapFileReader, MmapFileReaderExt};

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
    };
  }
}

#[cfg(feature = "smol")]
#[cfg_attr(docsrs, doc(cfg(feature = "smol")))]
pub(crate) mod smol_impl;

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub(crate) mod tokio_impl;
