cfg_sync!(
    mod sync_impl;
    pub use sync_impl::EmptyMmapFile;
);

cfg_async! {
    macro_rules! declare_and_impl_async_empty_mmap_file {
        () => {
            #[derive(Default, Clone)]
            pub struct AsyncEmptyMmapFile {
                inner: [u8; 0],
                path: PathBuf,
            }

            #[async_trait]
            impl AsyncMmapFileExt for AsyncEmptyMmapFile {
                #[inline]
                fn len(&self) -> usize {
                    0
                }

                #[inline]
                fn as_slice(&self) -> &[u8] {
                    &self.inner
                }

                #[inline]
                fn bytes(&self, _offset: usize, _sz: usize) -> Result<&[u8]> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }

                #[inline]
                fn path(&self) -> &Path {
                    self.path.as_path()
                }

                #[inline]
                fn is_exec(&self) -> bool {
                    false
                }

                #[inline]
                async fn metadata(&self) -> Result<MetaData> {
                    Ok(MetaData::empty(EmptyMetaData))
                }

                #[inline]
                fn copy_all_to_vec(&self) -> Vec<u8> {
                    self.inner.to_vec()
                }

                #[inline]
                fn copy_range_to_vec(&self, _offset: usize, _len: usize) -> Vec<u8> {
                    self.inner.to_vec()
                }

                #[inline]
                async fn write_all_to_new_file<P: AsRef<Path> + Send>(&self, _new_file_path: P) -> Result<()> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }

                #[inline]
                async fn write_range_to_new_file<P: AsRef<Path> + Send>(&self, _new_file_path: P, _offset: usize, _sz: usize) -> Result<()> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }

                #[inline]
                fn reader(&self, _offset: usize) -> Result<AsyncMmapFileReader> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }

                #[inline]
                fn range_reader(&self, _offset: usize, _len: usize) -> Result<AsyncMmapFileReader> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }

                noop_file_lock!();

                #[inline]
                fn read_exact(&self, _dst: &mut [u8], _offset: usize) -> Result<()> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }

                #[inline]
                fn read_i8(&self, _offset: usize) -> Result<i8> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }

                #[inline]
                fn read_u8(&self, _offset: usize) -> Result<u8> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }
            }

            #[async_trait]
            impl AsyncMmapFileMutExt for AsyncEmptyMmapFile {
                #[inline]
                fn as_mut_slice(&mut self) -> &mut [u8] {
                    &mut self.inner
                }

                #[inline]
                fn is_cow(&self) -> bool {
                    false
                }

                #[inline]
                fn bytes_mut(&mut self, _offset: usize, _len: usize) -> Result<&mut [u8]> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }

                #[inline]
                fn zero_range(&mut self, _start: usize, _end: usize) {}

                noop_flush!();

                #[inline]
                async fn truncate(&mut self, _max_sz: u64) -> Result<()> {
                    Ok(())
                }

                #[inline]
                async fn remove(self) -> Result<()> {
                    Ok(())
                }

                #[inline]
                async fn close_with_truncate(self, _max_sz: i64) -> Result<()> {
                    Ok(())
                }

                #[inline]
                fn writer(&mut self, _offset: usize) -> Result<AsyncMmapFileWriter> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }

                #[inline]
                fn range_writer(&mut self, _offset: usize, _len: usize) -> Result<AsyncMmapFileWriter> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }

                #[inline]
                fn write(&mut self, _src: &[u8], _offset: usize) -> usize { 0 }

                #[inline]
                fn write_all(&mut self, _src: &[u8], _offset: usize) -> Result<()> {
                    Err(Error::from(ErrorKind::InvokeEmptyMmap))
                }
            }
        };
    }

    macro_rules! test_empty_mmap_file {
        ($attr: meta) => {
            #[cfg(test)]
            mod tests {
                use super::*;

                #[$attr]
                async fn test_async_empty() {
                    let mut file = AsyncEmptyMmapFile::default();
                    file.slice(0, 0);
                    file.as_slice();
                    file.as_mut_slice();
                    file.bytes(0,0).unwrap_err();
                    file.bytes_mut(0,0).unwrap_err();
                    file.metadata().await.unwrap();
                    file.copy_range_to_vec(0,0);
                    file.copy_all_to_vec();
                    file.write_all_to_new_file("test").await.unwrap_err();
                    file.write_range_to_new_file("test", 0, 0).await.unwrap_err();
                    assert!(!file.is_exec());
                    assert!(!file.is_cow());
                    assert_eq!(file.len(), 0);
                    file.path();
                    file.path_lossy();
                    file.path_string();
                    file.flush().unwrap();
                    file.flush_async().unwrap();
                    file.flush_range(0, 0).unwrap();
                    file.flush_async_range(0, 0).unwrap();
                    let mut buf = [0; 10];
                    file.reader(0).unwrap_err();
                    file.range_reader(0, 0).unwrap_err();
                    file.read_i8(0).unwrap_err();
                    file.read_u8(0).unwrap_err();
                    file.read_exact(&mut buf, 0).unwrap_err();
                    file.write(&buf, 0);
                    file.write_all(&[0], 0).unwrap_err();
                    file.writer(0).unwrap_err();
                    file.range_writer(0, 0).unwrap_err();
                    file.zero_range(0, 0);
                    file.clone().close_with_truncate(0).await.unwrap();
                    file.truncate(0).await.unwrap();
                    file.clone().remove().await.unwrap();
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
