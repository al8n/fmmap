use std::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use memmap2::MmapOptions;
use crate::error::Error;
use crate::MmapFileMut;
use crate::raw::DiskMmapFileMut;

declare_and_impl_options!(Options, OpenOptions);

pub trait OptionsExt {

}

impl Options {
    /// Create a new file and mmap this file with [`Options`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{Options, MmapFileMut, MmapFileMutExt, MmapFileExt};
    /// # use scopeguard::defer;
    ///
    /// let mut file = Options::new().max_size(100).create_mmap_file_mut("create_with_options_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("create_with_options_test.txt").unwrap());
    /// assert!(!file.is_empty());
    ///
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn create_mmap_file_mut<P: AsRef<Path>>(self, path: P) -> Result<MmapFileMut, Error> {
        Ok(MmapFileMut::from(DiskMmapFileMut::create_with_options(path, self)?))
    }


    /// Open or Create(if not exists) a file and mmap this file with [`Options`].
    ///
    /// # Examples
    ///
    /// File already exists
    ///
    /// ```rust
    /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt, Options};
    /// use std::fs::File;
    /// use std::io::{Read, Seek, SeekFrom, Write};
    /// # use scopeguard::defer;
    ///
    /// # let mut file = File::create("open_test_with_options.txt").unwrap();
    /// # defer!(std::fs::remove_file("open_test_with_options.txt").unwrap());
    /// # file.write_all("sanity text".as_bytes()).unwrap();
    /// # file.write_all("some data...".as_bytes()).unwrap();
    /// # drop(file);
    ///
    /// // mmap the file with options
    /// let mut file = Options::new()
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
    ///     .open_mmap_file_mut("open_test_with_options.txt")
    ///     .unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate(("some modified data...".len() + "sanity text".len()) as u64).unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("open_test_with_options.txt").unwrap();
    /// file.seek(SeekFrom::Start("sanity text".as_bytes().len() as u64)).unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// File does not exists
    ///
    /// ```no_run
    /// use fmmap::{MmapFileMut, MmapFileExt, MmapFileMutExt, Options};
    /// use std::fs::File;
    /// use std::io::{Read, Write};
    /// # use scopeguard::defer;
    ///
    /// // mmap the file with options
    /// let mut file = Options::new()
    ///     // allow read
    ///     .read(true)
    ///     // allow write
    ///     .write(true)
    ///     // allow append
    ///     .append(true)
    ///     // truncate to 100
    ///     .max_size(100)
    ///     .open_mmap_file_mut("open_test_with_options.txt")
    ///     .unwrap();
    ///
    /// # defer!(std::fs::remove_file("open_test_with_options.txt").unwrap());
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    ///
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate("some modified data...".len() as u64).unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// drop(file);
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("open_test_with_options.txt").unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_mmap_file_mut<P: AsRef<Path>>(self, path: P) -> Result<MmapFileMut, Error> {
        Ok(MmapFileMut::from(DiskMmapFileMut::open_with_options(path, self)?))
    }
}