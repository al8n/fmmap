macro_rules! read_impl {
    ($this:ident, $offset: tt, $typ:tt::$conv:tt) => {{
        const SIZE: usize = mem::size_of::<$typ>();
        // try to convert directly from the bytes
        // this Option<ret> trick is to avoid keeping a borrow on self
        // when advance() is called (mut borrow) and to call bytes() only once
        let mut buf = [0; SIZE];
        $this
            .read_exact(&mut buf, $offset)
            .map(|_| unsafe { $typ::$conv(*(&buf as *const _ as *const [_; SIZE])) })
    }};
}

macro_rules! impl_from {
    ($outer: ident, $enum_inner: ident, [$($inner: ident), +$(,)?]) => {
        $(
        impl From<$inner> for $outer {
            fn from(file: $inner) -> Self {
                $outer{ inner: <$enum_inner>::from(file) }
            }
        }
        )*
    };
}

macro_rules! impl_from_mut {
    ($outer: ident, $enum_inner: ident, [$($inner: ident), +$(,)?]) => {
        $(
        impl From<$inner> for $outer {
            fn from(file: $inner) -> Self {
                $outer{
                    inner: <$enum_inner>::from(file),
                    remove_on_drop: false,
                    deleted: false,
                }
            }
        }
        )*
    };
}

macro_rules! impl_drop {
    ($name: ident, $inner: ident, $empty: ident) => {
        impl Drop for $name {
            fn drop(&mut self) {
                if self.remove_on_drop && !self.deleted {
                    let empty = <$inner>::Empty(<$empty>::default());
                    // swap the inner to empty
                    let inner = mem::replace(&mut self.inner, empty);
                    // do remove and ignore the result
                    let path = inner.path_buf();
                    drop(inner);
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    };
}

macro_rules! impl_flush {
    () => {
        fn flush(&self) -> Result<()> {
            self.inner.flush()
        }

        fn flush_async(&self) -> Result<()> {
            self.inner.flush_async()
        }

        fn flush_range(&self, offset: usize, len: usize) -> Result<()> {
            self.inner.flush_range(offset, len)
        }

        fn flush_async_range(&self, offset: usize, len: usize) -> Result<()> {
            self.inner.flush_async_range(offset, len)
        }
    };
}

macro_rules! impl_read_ext {
    () => {
        /// Read bytes to the dst buf from the offset, returns how many bytes read.
        fn read(&self, dst: &mut [u8], offset: usize) -> usize {
            let buf = self.as_slice();

            if buf.len() < offset {
                0
            } else {
                let remaining = buf.len() - offset;
                let dst_len = dst.len();
                if remaining > dst_len {
                    dst.copy_from_slice(&buf[offset..offset + dst_len]);
                    dst_len
                } else {
                    dst.copy_from_slice(&buf[offset..offset + remaining]);
                    remaining
                }
            }
        }

        /// Read the exact number of bytes required to fill buf.
        fn read_exact(&self, dst: &mut [u8], offset: usize) -> Result<()> {
            let buf = self.as_slice();
            let remaining = buf.len().checked_sub(offset);
            match remaining {
                None => Err(Error::EOF),
                Some(remaining) => {
                    let dst_len = dst.len();
                    if remaining < dst_len {
                        Err(Error::EOF)
                    } else {
                        dst.copy_from_slice(&buf[offset..offset + dst_len]);
                        Ok(())
                    }
                }
            }
        }

        /// Read a signed 8 bit integer from offset.
        fn read_i8(&self, offset: usize) -> Result<i8> {
            let buf = self.as_slice();

            let remaining = buf.len().checked_sub(offset);
            match remaining {
                None => Err(Error::EOF),
                Some(remaining) => {
                    if remaining < 1 {
                        Err(Error::EOF)
                    } else {
                        Ok(buf[offset] as i8)
                    }
                }
            }
        }

        /// Read a signed 16 bit integer from offset in big-endian byte order.
        fn read_i16(&self, offset: usize) -> Result<i16> {
            read_impl!(self, offset, i16::from_be_bytes)
        }

        /// Read a signed 16 bit integer from offset in little-endian byte order.
        fn read_i16_le(&self, offset: usize) -> Result<i16> {
            read_impl!(self, offset, i16::from_le_bytes)
        }

        /// Read a signed integer from offset in big-endian byte order.
        fn read_isize(&self, offset: usize) -> Result<isize> {
            read_impl!(self, offset, isize::from_be_bytes)
        }

        /// Read a signed integer from offset in little-endian byte order.
        fn read_isize_le(&self, offset: usize) -> Result<isize> {
            read_impl!(self, offset, isize::from_le_bytes)
        }

        /// Read a signed 32 bit integer from offset in big-endian byte order.
        fn read_i32(&self, offset: usize) -> Result<i32> {
            read_impl!(self, offset, i32::from_be_bytes)
        }

        /// Read a signed 32 bit integer from offset in little-endian byte order.
        fn read_i32_le(&self, offset: usize) -> Result<i32> {
            read_impl!(self, offset, i32::from_le_bytes)
        }

        /// Read a signed 64 bit integer from offset in big-endian byte order.
        fn read_i64(&self, offset: usize) -> Result<i64> {
            read_impl!(self, offset, i64::from_be_bytes)
        }

        /// Read a signed 64 bit integer from offset in little-endian byte order.
        fn read_i64_le(&self, offset: usize) -> Result<i64> {
            read_impl!(self, offset, i64::from_le_bytes)
        }

        /// Read a signed 128 bit integer from offset in big-endian byte order.
        fn read_i128(&self, offset: usize) -> Result<i128> {
            read_impl!(self, offset, i128::from_be_bytes)
        }

        /// Read a signed 128 bit integer from offset in little-endian byte order.
        fn read_i128_le(&self, offset: usize) -> Result<i128> {
            read_impl!(self, offset, i128::from_le_bytes)
        }

        /// Read an unsigned 8 bit integer from offset.
        fn read_u8(&self, offset: usize) -> Result<u8> {
            let buf = self.as_slice();

            let remaining = buf.len().checked_sub(offset);
            match remaining {
                None => Err(Error::EOF),
                Some(remaining) => {
                    if remaining < 1 {
                        Err(Error::EOF)
                    } else {
                        Ok(buf[offset])
                    }
                }
            }
        }

        /// Read an unsigned 16 bit integer from offset in big-endian.
        fn read_u16(&self, offset: usize) -> Result<u16> {
            read_impl!(self, offset, u16::from_be_bytes)
        }

        /// Read an unsigned 16 bit integer from offset in little-endian.
        fn read_u16_le(&self, offset: usize) -> Result<u16> {
            read_impl!(self, offset, u16::from_le_bytes)
        }

        /// Read an unsigned integer from offset in big-endian byte order.
        fn read_usize(&self, offset: usize) -> Result<usize> {
            read_impl!(self, offset, usize::from_be_bytes)
        }

        /// Read an unsigned integer from offset in little-endian byte order.
        fn read_usize_le(&self, offset: usize) -> Result<usize> {
            read_impl!(self, offset, usize::from_le_bytes)
        }

        /// Read an unsigned 32 bit integer from offset in big-endian.
        fn read_u32(&self, offset: usize) -> Result<u32> {
            read_impl!(self, offset, u32::from_be_bytes)
        }

        /// Read an unsigned 32 bit integer from offset in little-endian.
        fn read_u32_le(&self, offset: usize) -> Result<u32> {
            read_impl!(self, offset, u32::from_le_bytes)
        }

        /// Read an unsigned 64 bit integer from offset in big-endian.
        fn read_u64(&self, offset: usize) -> Result<u64> {
            read_impl!(self, offset, u64::from_be_bytes)
        }

        /// Read an unsigned 64 bit integer from offset in little-endian.
        fn read_u64_le(&self, offset: usize) -> Result<u64> {
            read_impl!(self, offset, u64::from_le_bytes)
        }

        /// Read an unsigned 128 bit integer from offset in big-endian.
        fn read_u128(&self, offset: usize) -> Result<u128> {
            read_impl!(self, offset, u128::from_be_bytes)
        }

        /// Read an unsigned 128 bit integer from offset in little-endian.
        fn read_u128_le(&self, offset: usize) -> Result<u128> {
            read_impl!(self, offset, u128::from_le_bytes)
        }

        /// Read an IEEE754 single-precision (4 bytes) floating point number from
        /// offset in big-endian byte order.
        fn read_f32(&self, offset: usize) -> Result<f32> {
            read_impl!(self, offset, f32::from_be_bytes)
        }

        /// Read an IEEE754 single-precision (4 bytes) floating point number from
        /// offset in little-endian byte order.
        fn read_f32_le(&self, offset: usize) -> Result<f32> {
            read_impl!(self, offset, f32::from_le_bytes)
        }

        /// Read an IEEE754 single-precision (8 bytes) floating point number from
        /// offset in big-endian byte order.
        fn read_f64(&self, offset: usize) -> Result<f64> {
            read_impl!(self, offset, f64::from_be_bytes)
        }

        /// Read an IEEE754 single-precision (8 bytes) floating point number from
        /// offset in little-endian byte order.
        fn read_f64_le(&self, offset: usize) -> Result<f64> {
            read_impl!(self, offset, f64::from_le_bytes)
        }
    };
}

macro_rules! impl_write_ext {
    () => {
        /// Write bytes to the mmap from the offset.
        fn write(&mut self, src: &[u8], offset: usize) -> usize {
            let buf = self.as_mut_slice();
            if buf.len() <= offset {
                0
            } else {
                let remaining = buf.len() - offset;
                let src_len = src.len();
                if remaining > src_len {
                    buf[offset..offset + src_len].copy_from_slice(src);
                    src_len
                } else {
                    buf[offset..offset + remaining].copy_from_slice(&src[..remaining]);
                    remaining
                }
            }
        }

        /// Write the all of bytes in `src` to the mmap from the offset.
        fn write_all(&mut self, src: &[u8], offset: usize) -> Result<()> {
            let buf = self.as_mut_slice();
            let remaining = buf.len().checked_sub(offset);
            match remaining {
                None => Err(Error::EOF),
                Some(remaining) => {
                    let src_len = src.len();
                    if remaining < src_len {
                        Err(Error::EOF)
                    } else {
                        buf[offset..offset + src_len].copy_from_slice(src);
                        Ok(())
                    }
                }
            }
        }

        /// Writes a signed 8 bit integer to mmap from the offset.
        fn write_i8(&mut self, val: i8, offset: usize) -> Result<()> {
            self.write_all(&[val as u8], offset)
        }

        /// Writes a signed 16 bit integer to mmap from the offset in the big-endian byte order.
        fn write_i16(&mut self, val: i16, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes a signed 16 bit integer to mmap from the offset in the little-endian byte order.
        fn write_i16_le(&mut self, val: i16, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes a signed integer to mmap from the offset in the big-endian byte order.
        fn write_isize(&mut self, val: isize, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes a signed integer to mmap from the offset in the little-endian byte order.
        fn write_isize_le(&mut self, val: isize, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes a signed 32 bit integer to mmap from the offset in the big-endian byte order.
        fn write_i32(&mut self, val: i32, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes a signed 32 bit integer to mmap from the offset in the little-endian byte order.
        fn write_i32_le(&mut self, val: i32, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes a signed 64 bit integer to mmap from the offset in the big-endian byte order.
        fn write_i64(&mut self, val: i64, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes a signed 64 bit integer to mmap from the offset in the little-endian byte order.
        fn write_i64_le(&mut self, val: i64, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes a signed 128 bit integer to mmap from the offset in the big-endian byte order.
        fn write_i128(&mut self, val: i128, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes a signed 128 bit integer to mmap from the offset in the little-endian byte order.
        fn write_i128_le(&mut self, val: i128, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an unsigned 8 bit integer to mmap from the offset.
        fn write_u8(&mut self, val: u8, offset: usize) -> Result<()> {
            self.write_all(&[val], offset)
        }

        /// Writes an unsigned 16 bit integer to mmap from the offset in the big-endian byte order.
        fn write_u16(&mut self, val: u16, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an unsigned 16 bit integer to mmap from the offset in the little-endian byte order.
        fn write_u16_le(&mut self, val: u16, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an unsigned integer to mmap from the offset in the big-endian byte order.
        fn write_usize(&mut self, val: usize, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an unsigned integer to mmap from the offset in the little-endian byte order.
        fn write_usize_le(&mut self, val: usize, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an unsigned 32 bit integer to mmap from the offset in the big-endian byte order.
        fn write_u32(&mut self, val: u32, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an unsigned 32 bit integer to mmap from the offset in the little-endian byte order.
        fn write_u32_le(&mut self, val: u32, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an unsigned 64 bit integer to mmap from the offset in the big-endian byte order.
        fn write_u64(&mut self, val: u64, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an unsigned 64 bit integer to mmap from the offset in the little-endian byte order.
        fn write_u64_le(&mut self, val: u64, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an unsigned 128 bit integer to mmap from the offset in the big-endian byte order.
        fn write_u128(&mut self, val: u128, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an unsigned 128 bit integer to mmap from the offset in the little-endian byte order.
        fn write_u128_le(&mut self, val: u128, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an IEEE754 single-precision (4 bytes) floating point number to mmap from the offset in big-endian byte order.
        fn write_f32(&mut self, val: f32, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an IEEE754 single-precision (4 bytes) floating point number to mmap from the offset in little-endian byte order.
        fn write_f32_le(&mut self, val: f32, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }

        /// Writes an IEEE754 single-precision (8 bytes) floating point number to mmap from the offset in big-endian byte order.
        fn write_f64(&mut self, val: f64, offset: usize) -> Result<()> {
            self.write_all(&val.to_be_bytes(), offset)
        }

        /// Writes an IEEE754 single-precision (8 bytes) floating point number to mmap from the offset in little-endian byte order.
        fn write_f64_le(&mut self, val: f64, offset: usize) -> Result<()> {
            self.write_all(&val.to_le_bytes(), offset)
        }
    };
}

cfg_sync!(
    macro_rules! impl_mmap_file_ext {
        ($name: ident) => {
            impl MmapFileExt for $name {
                #[inline]
                fn len(&self) -> usize {
                    self.inner.len()
                }

                #[inline]
                fn as_slice(&self) -> &[u8] {
                    self.inner.as_slice()
                }

                #[inline]
                fn path(&self) -> &Path {
                    self.inner.path()
                }

                #[inline]
                fn is_exec(&self) -> bool {
                    self.inner.is_exec()
                }

                #[inline]
                fn metadata(&self) -> Result<MetaData> {
                    self.inner.metadata()
                }
            }
        };
    }

    mod sync_impl;
    pub use sync_impl::{MmapFileExt, MmapFileMutExt, MmapFile, MmapFileMut};
);

cfg_tokio!(
    macro_rules! impl_async_mmap_file_ext {
        ($name: ident) => {
            #[async_trait]
            impl AsyncMmapFileExt for $name {
                #[inline]
                fn len(&self) -> usize {
                    self.inner.len()
                }

                #[inline]
                fn as_slice(&self) -> &[u8] {
                    self.inner.as_slice()
                }

                #[inline]
                fn path(&self) -> &Path {
                    self.inner.path()
                }

                #[inline]
                fn is_exec(&self) -> bool {
                    self.inner.is_exec()
                }

                #[inline]
                async fn metadata(&self) -> Result<MetaData> {
                    self.inner.metadata().await
                }
            }
        };
    }

    mod tokio_impl;
    pub use tokio_impl::{AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncMmapFile, AsyncMmapFileMut};
);
