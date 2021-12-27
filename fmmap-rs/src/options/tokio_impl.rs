use std::path::Path;
use tokio::fs::OpenOptions;
use memmap2::MmapOptions;
use crate::{AsyncMmapFile, AsyncMmapFileMut};
use crate::error::Error;
use crate::raw::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};

declare_and_impl_options!(AsyncOptions, OpenOptions);

impl AsyncOptions {
    /// Create a new file and mmap this file with [`AsyncOptions`]
    ///
    /// ```rust
    /// use fmmap::{AsyncMmapFileMut, AsyncOptions, AsyncMmapFileMutExt, AsyncMmapFileExt};
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// let mut file = AsyncOptions::new()
    ///     // truncate to 100
    ///     .max_size(100)
    ///     .create_mmap_file_mut("async_create_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_create_with_options_test.txt").unwrap());
    /// assert!(!file.is_empty());
    ///
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn create_mmap_file_mut<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFileMut, Error> {
        Ok(AsyncMmapFileMut::from(AsyncDiskMmapFileMut::create_with_options(path, self).await?))
    }

    /// Open a readable memory map backed by a file with [`Options`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{AsyncOptions, AsyncMmapFile, AsyncMmapFileExt};
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_open_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_with_options_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    ///
    /// // mmap the file
    /// let file = AsyncOptions::new()
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64)
    ///     .open_mmap_file("async_open_with_options_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open_mmap_file<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFile, Error> {
        Ok(AsyncMmapFile::from(AsyncDiskMmapFile::open_with_options(path, self).await?))
    }

    /// Open a readable and executable memory map backed by a file with [`AsyncOptions`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{AsyncMmapFile, AsyncOptions, AsyncMmapFileExt};
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_open_exec_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_exec_with_options_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    ///
    /// // mmap the file
    /// let file = AsyncOptions::new()
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64)
    ///     .open_exec_mmap_file("async_open_exec_with_options_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open_exec_mmap_file<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFile, Error> {
        Ok(AsyncMmapFile::from(AsyncDiskMmapFile::open_exec_with_options(path, self).await?))
    }

    /// Open or Create(if not exists) a file and mmap this file with [`AsyncOptions`].
    ///
    /// # Examples
    ///
    /// File already exists
    ///
    /// ```rust
    /// use fmmap::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use tokio::fs::File;
    /// use std::io::SeekFrom;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// # let mut file = File::create("async_open_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_with_options_test.txt").unwrap());
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// # tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// # drop(file);
    ///
    /// // mmap the file
    /// let mut file = AsyncOptions::new()
    ///     // allow read
    ///     .read(true)
    ///     // allow write
    ///     .write(true)
    ///     // allow append
    ///     .append(true)
    ///     // truncate to 100
    ///     .max_size(100)
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64)
    ///     .open_mmap_file_mut("async_open_with_options_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).await.unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("async_open_with_options_test.txt").await.unwrap();
    /// // skip the sanity text
    /// tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start("sanity text".as_bytes().len() as u64)).await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// File does not exists
    ///
    /// ```no_run
    /// use fmmap::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use tokio::fs::File;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // mmap the file with options
    /// let mut file = AsyncOptions::new()
    ///     // allow read
    ///     .read(true)
    ///     // allow write
    ///     .write(true)
    ///     // allow append
    ///     .append(true)
    ///     // truncate to 100
    ///     .max_size(100)
    ///     .open_mmap_file_mut("async_open_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_with_options_test.txt").unwrap());
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate("some modified data...".len() as u64).await.unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open(".txt").await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open_mmap_file_mut<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFileMut, Error> {
        Ok(AsyncMmapFileMut::from(AsyncDiskMmapFileMut::open_with_options(path, self).await?))
    }

    /// Open an existing file and mmap this file with [`AsyncOptions`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use tokio::fs::File;
    /// use std::io::SeekFrom;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // create a temp file
    /// let mut file = File::create("async_open_existing_test_with_options.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_existing_test_with_options.txt").unwrap());
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = AsyncOptions::new()
    ///     // truncate to 100
    ///     .max_size(100)
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64)
    ///     .open_exist_mmap_file_mut("async_open_existing_test_with_options.txt").await.unwrap();
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).await.unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    ///
    /// // reopen to check content, cow will not change the content.
    /// let mut file = File::open("async_open_existing_test_with_options.txt").await.unwrap();
    /// let mut buf = vec![0; "some modified data...".len()];
    /// // skip the sanity text
    /// tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start("sanity text".as_bytes().len() as u64)).await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open_exist_mmap_file_mut<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFileMut, Error> {
        Ok(AsyncMmapFileMut::from(AsyncDiskMmapFileMut::open_exist_with_options(path, self).await?))
    }

    /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file) with [`AsyncOptions`].
    /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{AsyncMmapFileMut, AsyncMmapFileExt, AsyncMmapFileMutExt, AsyncOptions};
    /// use tokio::fs::File;
    /// use std::io::SeekFrom;
    /// # use scopeguard::defer;
    ///
    /// # tokio_test::block_on(async {
    /// // create a temp file
    /// let mut file = File::create("async_open_cow_with_options_test.txt").await.unwrap();
    /// # defer!(std::fs::remove_file("async_open_cow_with_options_test.txt").unwrap());
    ///
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "sanity text".as_bytes()).await.unwrap();
    /// tokio::io::AsyncWriteExt::write_all(&mut file, "some data...".as_bytes()).await.unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = AsyncOptions::new()
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64)
    ///     .open_cow_mmap_file_mut("async_open_cow_with_options_test.txt").await.unwrap();
    /// assert!(file.is_cow());
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.write_all("some data!!!".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// // cow, change will only be seen in current caller
    /// assert_eq!(file.as_slice(), "some data!!!".as_bytes());
    /// drop(file);
    ///
    /// // reopen to check content, cow will not change the content.
    /// let mut file = File::open("async_open_cow_with_options_test.txt").await.unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// // skip the sanity text
    /// tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start("sanity text".as_bytes().len() as u64)).await.unwrap();
    /// tokio::io::AsyncReadExt::read_exact(&mut file, buf.as_mut_slice()).await.unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// # })
    /// ```
    ///
    /// [`AsyncOptions`]: struct.AsyncOptions.html
    pub async fn open_cow_mmap_file_mut<P: AsRef<Path>>(self, path: P) -> Result<AsyncMmapFileMut, Error> {
        Ok(AsyncMmapFileMut::from(AsyncDiskMmapFileMut::open_cow_with_options(path, self).await?))
    }
}