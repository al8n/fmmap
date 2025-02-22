use crate::disk::MmapFileMutType;
use crate::error::{Error, ErrorKind};
use crate::options::Options;
use crate::utils::{
    create_file, open_exist_file_with_append, open_or_create_file, open_read_only_file, sync_parent,
};
use crate::{MetaData, MmapFileExt, MmapFileMutExt};
use fs4::fs_std::FileExt;
use memmap2::{Mmap, MmapAsRawDesc, MmapMut, MmapOptions};
use std::fs::{remove_file, File};
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "linux"))]
use std::ptr::{drop_in_place, write};

remmap!(Path);

/// DiskMmapFile contains an immutable mmap buffer
/// and a read-only file.
pub struct DiskMmapFile {
    pub(crate) mmap: Mmap,
    pub(crate) file: File,
    pub(crate) path: PathBuf,
    exec: bool,
}

impl_mmap_file_ext!(DiskMmapFile);

impl DiskMmapFile {
    /// Open a readable memory map backed by a file
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use fmmap::MmapFileExt;
    /// use fmmap::raw::DiskMmapFile;
    /// use std::fs::{remove_file, File};
    /// use std::io::Write;
    /// # use scopeguard::defer;
    ///
    /// # let mut file = File::create("disk_open_test.txt").unwrap();
    /// # defer!(remove_file("disk_open_test.txt").unwrap());
    /// # file.write_all("some data...".as_bytes()).unwrap();
    /// # drop(file);
    /// // open and mmap the file
    /// let mut file = DiskMmapFile::open("disk_open_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_in(path, None)
    }

    /// Open a readable memory map backed by a file with [`Options`]
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use fmmap::{Options, MmapFileExt};
    /// use fmmap::raw::DiskMmapFile;
    /// use std::fs::File;
    /// use std::io::Write;
    /// # use scopeguard::defer;
    ///
    /// # let mut file = File::create("disk_open_test_with_options.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_open_test_with_options.txt").unwrap());
    /// # file.write_all("sanity text".as_bytes()).unwrap();
    /// # file.write_all("some data...".as_bytes()).unwrap();
    /// # drop(file);
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
    ///     // allow read
    ///     .read(true)
    ///     // allow write
    ///     .write(true)
    ///     // allow append
    ///     .append(true)
    ///     // truncate to 100
    ///     .max_size(100)
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// // open and mmap the file
    /// let mut file = DiskMmapFile::open_with_options("disk_open_test_with_options.txt", opts).unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
        Self::open_in(path, Some(opts))
    }

    /// Open a readable and executable memory map backed by a file
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use fmmap::MmapFileExt;
    /// use fmmap::raw::DiskMmapFile;
    /// use std::fs::{remove_file, File};
    /// use std::io::Write;
    /// # use scopeguard::defer;
    ///
    /// # let mut file = File::create("disk_open_exec_test.txt").unwrap();
    /// # defer!(remove_file("disk_open_exec_test.txt").unwrap());
    /// # file.write_all("some data...".as_bytes()).unwrap();
    /// # drop(file);
    /// // open and mmap the file
    /// let mut file = DiskMmapFile::open_exec("disk_open_exec_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    pub fn open_exec<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_exec_in(path, None)
    }

    /// Open a readable and executable memory map backed by a file with [`Options`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use fmmap::{Options, MmapFileExt};
    /// use fmmap::raw::DiskMmapFile;
    /// use std::fs::File;
    /// use std::io::Write;
    /// # use scopeguard::defer;
    ///
    /// # let mut file = File::create("disk_open_exec_test_with_options.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_open_exec_test_with_options.txt").unwrap());
    /// # file.write_all("sanity text".as_bytes()).unwrap();
    /// # file.write_all("some data...".as_bytes()).unwrap();
    /// # drop(file);
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
    ///     // allow read
    ///     .read(true)
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// // open and mmap the file
    /// let mut file = DiskMmapFile::open_exec_with_options("disk_open_exec_test_with_options.txt", opts).unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0);
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_exec_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
        Self::open_exec_in(path, Some(opts))
    }

    fn open_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
        let file = open_read_only_file(&path).map_err(|e| {
            Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e)
        })?;
        match opts {
            None => {
                let mmap =
                    unsafe { Mmap::map(&file).map_err(|e| Error::new(ErrorKind::MmapFailed, e))? };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    exec: false,
                })
            }
            Some(opts) => {
                let mmap = unsafe {
                    opts.mmap_opts
                        .map(&file)
                        .map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    exec: false,
                })
            }
        }
    }

    fn open_exec_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
        let file = open_read_only_file(&path).map_err(|e| {
            Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e)
        })?;

        match opts {
            None => {
                let mmap = unsafe {
                    MmapOptions::new()
                        .map_exec(&file)
                        .map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    exec: true,
                })
            }
            Some(opts) => {
                let mmap = unsafe {
                    opts.mmap_opts
                        .map_exec(&file)
                        .map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    exec: true,
                })
            }
        }
    }
}

/// DiskMmapFile contains a mutable mmap buffer
/// and a writable file.
pub struct DiskMmapFileMut {
    pub(crate) mmap: MmapMut,
    pub(crate) file: File,
    pub(crate) path: PathBuf,
    opts: Option<MmapOptions>,
    typ: MmapFileMutType,
}

impl_mmap_file_ext_for_mut!(DiskMmapFileMut);

impl MmapFileMutExt for DiskMmapFileMut {
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.mmap.as_mut()
    }

    fn is_cow(&self) -> bool {
        matches!(self.typ, MmapFileMutType::Cow)
    }

    impl_flush!();

    #[cfg(not(target_os = "linux"))]
    fn truncate(&mut self, max_sz: u64) -> Result<(), Error> {
        if self.is_cow() {
            return Err(Error::new_with_message(
                ErrorKind::TruncationFailed,
                "cannot truncate a copy-on-write mmap file",
            ));
        }

        // sync data
        let meta = self
            .file
            .metadata()
            .map_err(|e| Error::new(ErrorKind::IO, e))?;
        if meta.len() > 0 {
            self.flush()?;
        }

        unsafe {
            // unmap
            drop_in_place(&mut self.mmap);

            // truncate
            self.file.set_len(max_sz).map_err(|e| {
                Error::new_source_msg(ErrorKind::TruncationFailed, self.path_string(), e)
            })?;

            // remap
            let mmap = remmap(self.path(), &self.file, self.opts.as_ref(), self.typ)?;

            write(&mut self.mmap, mmap);
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn truncate(&mut self, max_sz: u64) -> Result<(), Error> {
        if self.is_cow() {
            return Err(Error::new_with_message(
                ErrorKind::TruncationFailed,
                "cannot truncate a copy-on-write mmap file",
            ));
        }

        // sync data
        self.flush()?;

        // truncate
        self.file.set_len(max_sz).map_err(|e| {
            Error::new_source_msg(ErrorKind::TruncationFailed, self.path_string(), e)
        })?;

        // remap
        self.mmap = remmap(self.path(), &self.file, self.opts.as_ref(), self.typ)?;

        Ok(())
    }

    /// Remove the underlying file
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use fmmap::MmapFileMutExt;
    /// use fmmap::raw::DiskMmapFileMut;
    ///
    /// let mut file = DiskMmapFileMut::create("disk_remove_test.txt").unwrap();
    ///
    /// file.truncate(100);
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.remove().unwrap();
    ///
    /// let err = std::fs::File::open("disk_remove_test.txt");
    /// assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    /// ```
    fn drop_remove(self) -> crate::error::Result<()> {
        let path = self.path;
        drop(self.mmap);
        self.file
            .set_len(0)
            .map_err(|e| Error::new(ErrorKind::IO, e))?;
        drop(self.file);
        remove_file(path).map_err(|e| Error::new(ErrorKind::IO, e))
    }

    /// Close and truncate the underlying file
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use fmmap::{MetaDataExt, MmapFileExt, MmapFileMutExt};
    /// use fmmap::raw::DiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// let mut file = DiskMmapFileMut::create("disk_close_with_truncate_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_close_with_truncate_test.txt").unwrap());
    /// file.truncate(100);
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.close_with_truncate(50).unwrap();
    ///
    /// let file = DiskMmapFileMut::open("disk_close_with_truncate_test.txt").unwrap();
    /// let meta = file.metadata().unwrap();
    /// assert_eq!(meta.len(), 50);
    /// ```
    #[cfg(not(target_os = "linux"))]
    fn close_with_truncate(self, max_sz: i64) -> crate::error::Result<()> {
        // sync data
        let meta = self
            .file
            .metadata()
            .map_err(|e| Error::new(ErrorKind::IO, e))?;
        if meta.len() > 0 {
            self.flush()?;
        }

        drop(self.mmap);
        if max_sz >= 0 {
            self.file
                .set_len(max_sz as u64)
                .map_err(|e| Error::new(ErrorKind::IO, e))?;
            let abs = self
                .path
                .canonicalize()
                .map_err(|e| Error::new(ErrorKind::IO, e))?;
            let parent = abs.parent().unwrap();
            File::open(parent)
                .map_err(|e| Error::new(ErrorKind::IO, e))?
                .sync_all()
                .map_err(|e| Error::new(ErrorKind::SyncDirFailed, e))?
        }
        Ok(())
    }

    /// Close and truncate the underlying file
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use fmmap::{MetaDataExt, MmapFileExt, MmapFileMutExt};
    /// use fmmap::raw::DiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// let mut file = DiskMmapFileMut::create("disk_close_with_truncate_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_close_with_truncate_test.txt").unwrap());
    /// file.truncate(100);
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.close_with_truncate(50).unwrap();
    ///
    /// let file = DiskMmapFileMut::open("disk_close_with_truncate_test.txt").unwrap();
    /// let meta = file.metadata().unwrap();
    /// assert_eq!(meta.len(), 50);
    /// ```
    #[cfg(target_os = "linux")]
    fn close_with_truncate(self, max_sz: i64) -> crate::error::Result<()> {
        self.flush()?;
        drop(self.mmap);
        if max_sz >= 0 {
            self.file
                .set_len(max_sz as u64)
                .map_err(|e| Error::new(ErrorKind::IO, e))?;
            let abs = self
                .path
                .canonicalize()
                .map_err(|e| Error::new(ErrorKind::IO, e))?;
            let parent = abs.parent().unwrap();
            File::open(parent)
                .map_err(|e| Error::new(ErrorKind::IO, e))?
                .sync_all()
                .map_err(|e| Error::new(ErrorKind::SyncDirFailed, e))?
        }
        Ok(())
    }
}

impl DiskMmapFileMut {
    /// Create a new file and mmap this file
    ///
    /// # Notes
    /// The new file is zero size, so before do write, you should truncate first.
    /// Or you can use [`create_with_options`] and set `max_size` field for [`Options`] to enable directly write
    /// without truncating.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use fmmap::MmapFileMutExt;
    /// use fmmap::raw::DiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// let mut file = DiskMmapFileMut::create("disk_create_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_create_test.txt").unwrap());
    /// file.truncate(100);
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// ```
    ///
    /// [`create_with_options`]: struct.DiskMmapFileMut.html#method.create_with_options
    /// [`Options`]: struct.Options.html
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::create_in(path, None)
    }

    /// Create a new file and mmap this file with [`Options`]
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use fmmap::{Options, MmapFileMutExt};
    /// use fmmap::raw::DiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// let opts = Options::new()
    ///     // truncate to 100
    ///     .max_size(100);
    /// let mut file = DiskMmapFileMut::create_with_options("disk_create_with_options_test.txt", opts).unwrap();
    /// # defer!(std::fs::remove_file("disk_create_with_options_test.txt").unwrap());
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn create_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
        Self::create_in(path, Some(opts))
    }

    /// Open or Create(if not exists) a file and mmap this file.
    ///
    /// # Notes
    /// If the file does not exist, then the new file will be open in zero size, so before do write, you should truncate first.
    /// Or you can use [`open_with_options`] and set `max_size` field for [`Options`] to enable directly write
    /// without truncating.
    ///
    /// # Examples
    ///
    /// File already exists
    ///
    /// ```ignore
    /// use fmmap::{MmapFileExt, MmapFileMutExt};
    /// use fmmap::raw::DiskMmapFileMut;
    /// use std::fs::File;
    /// use std::io::{Read, Write};
    /// # use scopeguard::defer;
    ///
    /// # let mut file = File::create("disk_open_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_open_test.txt").unwrap());
    /// # file.write_all("some data...".as_bytes()).unwrap();
    /// # drop(file);
    ///
    /// // open and mmap the file
    /// let mut file = DiskMmapFileMut::open("disk_open_test.txt").unwrap();
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
    /// let mut file = File::open("disk_open_test.txt").unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// File does not exists
    ///
    /// ```ignore
    /// use fmmap::{MmapFileExt, MmapFileMutExt};
    /// use fmmap::raw::DiskMmapFileMut;
    /// use std::fs::File;
    /// use std::io::{Read, Write};
    /// # use scopeguard::defer;
    ///
    /// // create and mmap the file
    /// let mut file = DiskMmapFileMut::open("disk_open_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_open_test.txt").unwrap());
    /// file.truncate(100).unwrap();
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
    /// let mut file = File::open("disk_open_test.txt").unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// [`open_with_options`]: struct.DiskMmapFileMut.html#method.open_with_options
    /// [`Options`]: struct.Options.html
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_in(path, None)
    }

    /// Open or Create(if not exists) a file and mmap this file with [`Options`].
    ///
    /// # Examples
    ///
    /// File already exists
    ///
    /// ```ignore
    /// use fmmap::{MmapFileExt, MmapFileMutExt, Options};
    /// use fmmap::raw::DiskMmapFileMut;
    /// use std::fs::File;
    /// use std::io::{Read, Seek, SeekFrom, Write};
    /// # use scopeguard::defer;
    ///
    /// # let mut file = File::create("disk_open_test_with_options.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_open_test_with_options.txt").unwrap());
    /// # file.write_all("sanity text".as_bytes()).unwrap();
    /// # file.write_all("some data...".as_bytes()).unwrap();
    /// # drop(file);
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
    ///     // allow read
    ///     .read(true)
    ///     // allow write
    ///     .write(true)
    ///     // allow append
    ///     .append(true)
    ///     // truncate to 100
    ///     .max_size(100)
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// let mut file = DiskMmapFileMut::open_with_options("disk_open_test_with_options.txt", opts).unwrap();
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
    /// let mut file = File::open("disk_open_test_with_options.txt").unwrap();
    /// file.seek(SeekFrom::Start("sanity text".as_bytes().len() as u64)).unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// File does not exists
    ///
    /// ```ignore
    /// use fmmap::{MmapFileExt, MmapFileMutExt, Options};
    /// use fmmap::raw::DiskMmapFileMut;
    /// use std::fs::File;
    /// use std::io::{Read, Write};
    /// # use scopeguard::defer;
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
    ///     // allow read
    ///     .read(true)
    ///     // allow write
    ///     .write(true)
    ///     // allow append
    ///     .append(true)
    ///     // truncate to 100
    ///     .max_size(100);
    ///
    /// let mut file = DiskMmapFileMut::open_with_options("disk_open_test_with_options.txt", opts).unwrap();
    /// # defer!(std::fs::remove_file("disk_open_test_with_options.txt").unwrap());
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
    /// let mut file = File::open("disk_open_test_with_options.txt").unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
        Self::open_in(path, Some(opts))
    }

    /// Open an existing file and mmap this file
    ///
    /// # Examples
    /// ```ignore
    /// use fmmap::{MmapFileExt, MmapFileMutExt};
    /// use fmmap::raw::DiskMmapFileMut;
    /// use std::fs::File;
    /// use std::io::{Read, Write};
    /// # use scopeguard::defer;
    ///
    /// // create a temp file
    /// let mut file = File::create("disk_open_existing_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_open_existing_test.txt").unwrap());
    /// file.write_all("some data...".as_bytes()).unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = DiskMmapFileMut::open_exist("disk_open_existing_test.txt").unwrap();
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
    /// let mut file = File::open("disk_open_existing_test.txt").unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    pub fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_exist_in(path, None)
    }

    /// Open an existing file and mmap this file with [`Options`]
    ///
    /// # Examples
    /// ```ignore
    /// use fmmap::{MmapFileExt, MmapFileMutExt, Options};
    /// use fmmap::raw::DiskMmapFileMut;
    /// use std::fs::File;
    /// use std::io::{Read, Seek, SeekFrom, Write};
    /// # use scopeguard::defer;
    ///
    /// // create a temp file
    /// let mut file = File::create("disk_open_existing_test_with_options.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_open_existing_test_with_options.txt").unwrap());
    /// file.write_all("sanity text".as_bytes()).unwrap();
    /// file.write_all("some data...".as_bytes()).unwrap();
    /// drop(file);
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
    ///     // truncate to 100
    ///     .max_size(100)
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// let mut file = DiskMmapFileMut::open_exist_with_options("disk_open_existing_test_with_options.txt", opts).unwrap();
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
    /// let mut file = File::open("disk_open_existing_test_with_options.txt").unwrap();
    /// file.seek(SeekFrom::Start("sanity text".as_bytes().len() as u64)).unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
        Self::open_exist_in(path, Some(opts))
    }

    /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file).
    /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use fmmap::{MmapFileExt, MmapFileMutExt};
    /// use fmmap::raw::DiskMmapFileMut;
    /// use std::fs::File;
    /// use std::io::{Read, Write};
    /// # use scopeguard::defer;
    ///
    /// // create a temp file
    /// let mut file = File::create("disk_open_cow_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_open_cow_test.txt").unwrap());
    /// file.write_all("some data...".as_bytes()).unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = DiskMmapFileMut::open_cow("disk_open_cow_test.txt").unwrap();
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
    /// let mut file = File::open("disk_open_cow_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_cow_in(path, None)
    }

    /// Open and mmap an existing file in copy-on-write mode(copy-on-write memory map backed by a file) with [`Options`].
    /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use fmmap::{MmapFileExt, MmapFileMutExt, Options};
    /// use fmmap::raw::DiskMmapFileMut;
    /// use std::fs::File;
    /// use std::io::{Read, Seek, Write, SeekFrom};
    /// # use scopeguard::defer;
    ///
    /// // create a temp file
    /// let mut file = File::create("disk_open_cow_with_options_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_open_cow_with_options_test.txt").unwrap());
    /// file.write_all("sanity text".as_bytes()).unwrap();
    /// file.write_all("some data...".as_bytes()).unwrap();
    /// drop(file);
    ///
    /// // mmap the file with options
    /// let opts = Options::new()
    ///     // mmap content after the sanity text
    ///     .offset("sanity text".as_bytes().len() as u64);
    /// let mut file = DiskMmapFileMut::open_cow_with_options("disk_open_cow_with_options_test.txt", opts).unwrap();
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
    /// let mut file = File::open("disk_open_cow_with_options_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// // skip the sanity text
    /// file.seek(SeekFrom::Start("sanity text".as_bytes().len() as u64)).unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: struct.Options.html
    pub fn open_cow_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
        Self::open_cow_in(path, Some(opts))
    }

    /// Returns an immutable version of this memory mapped buffer.
    /// If the memory map is file-backed, the file must have been opened with read permissions.
    ///
    /// # Errors
    /// This method returns an error when the underlying system call fails,
    /// which can happen for a variety of reasons,
    /// such as when the file has not been opened with read permissions.
    ///
    /// # Examples
    /// ```ignore
    /// use fmmap::MmapFileMutExt;
    /// use fmmap::raw::DiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// let mut file = DiskMmapFileMut::create("disk_mmap_file_freeze_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_mmap_file_freeze_test.txt").unwrap());
    /// file.truncate(12);
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.freeze().unwrap();
    /// ```
    pub fn freeze(self) -> Result<DiskMmapFile, Error> {
        Ok(DiskMmapFile {
            mmap: self
                .mmap
                .make_read_only()
                .map_err(|e| Error::new(ErrorKind::IO, e))?,
            file: self.file,
            path: self.path,
            exec: false,
        })
    }

    /// Transition the memory map to be readable and executable.
    /// If the memory map is file-backed, the file must have been opened with execute permissions.
    ///
    /// # Errors
    /// This method returns an error when the underlying system call fails,
    /// which can happen for a variety of reasons,
    /// such as when the file has not been opened with execute permissions
    ///
    /// # Examples
    /// ```ignore
    /// use fmmap::MmapFileMutExt;
    /// use fmmap::raw::DiskMmapFileMut;
    /// # use scopeguard::defer;
    ///
    /// let mut file = DiskMmapFileMut::create("disk_mmap_file_freeze_test.txt").unwrap();
    /// # defer!(std::fs::remove_file("disk_mmap_file_freeze_test.txt").unwrap());
    /// file.truncate(12);
    /// file.write_all("some data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// file.freeze_exec().unwrap();
    /// ```
    pub fn freeze_exec(self) -> Result<DiskMmapFile, Error> {
        Ok(DiskMmapFile {
            mmap: self
                .mmap
                .make_exec()
                .map_err(|e| Error::new(ErrorKind::IO, e))?,
            file: self.file,
            path: self.path,
            exec: true,
        })
    }

    fn create_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
        let file = create_file(&path).map_err(|e| {
            Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e)
        })?;

        match opts {
            None => {
                let mmap = unsafe {
                    MmapMut::map_mut(&file).map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
                };

                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: None,
                    typ: MmapFileMutType::Normal,
                })
            }
            Some(opts) => {
                if opts.max_size > 0 {
                    file.set_len(opts.max_size).map_err(|e| {
                        Error::new_source_msg(
                            ErrorKind::TruncationFailed,
                            path.as_ref().to_string_lossy(),
                            e,
                        )
                    })?;
                    sync_parent(&path)?;
                }

                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe {
                    opts.mmap_opts
                        .map_mut(&file)
                        .map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
                };

                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: Some(opts_bk),
                    typ: MmapFileMutType::Normal,
                })
            }
        }
    }

    fn open_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
        match opts {
            None => {
                let file = open_or_create_file(&path).map_err(|e| {
                    Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e)
                })?;
                let mmap = unsafe {
                    MmapMut::map_mut(&file).map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: None,
                    typ: MmapFileMutType::Normal,
                })
            }
            Some(mut opts) => {
                let file = opts.file_opts.create(true).open(&path).map_err(|e| {
                    Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e)
                })?;
                let meta = file.metadata()?;
                let file_sz = meta.len();
                if file_sz == 0 && opts.max_size > 0 {
                    file.set_len(opts.max_size).map_err(|e| {
                        Error::new_source_msg(
                            ErrorKind::TruncationFailed,
                            path.as_ref().to_string_lossy(),
                            e,
                        )
                    })?;
                    sync_parent(&path)?;
                }

                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe {
                    opts.mmap_opts
                        .map_mut(&file)
                        .map_err(|e| Error::new(ErrorKind::MmapFailed, e))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: Some(opts_bk),
                    typ: MmapFileMutType::Normal,
                })
            }
        }
    }

    fn open_exist_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
        let file = open_exist_file_with_append(&path).map_err(|e| {
            Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e)
        })?;

        match opts {
            None => {
                let mmap = unsafe { MmapMut::map_mut(&file)? };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: None,
                    typ: MmapFileMutType::Normal,
                })
            }
            Some(opts) => {
                let meta = file.metadata()?;
                let file_sz = meta.len();
                if file_sz == 0 && opts.max_size > 0 {
                    file.set_len(opts.max_size).map_err(|e| {
                        Error::new_source_msg(
                            ErrorKind::TruncationFailed,
                            path.as_ref().to_string_lossy(),
                            e,
                        )
                    })?;
                    sync_parent(&path)?;
                }
                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe { opts.mmap_opts.map_mut(&file)? };

                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: Some(opts_bk),
                    typ: MmapFileMutType::Normal,
                })
            }
        }
    }

    fn open_cow_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
        let file = open_exist_file_with_append(&path).map_err(|e| {
            Error::new_source_msg(ErrorKind::OpenFailed, path.as_ref().to_string_lossy(), e)
        })?;

        match opts {
            None => {
                let mmap = unsafe { MmapOptions::new().map_copy(&file)? };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: None,
                    typ: MmapFileMutType::Cow,
                })
            }
            Some(opts) => {
                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe { opts.mmap_opts.map_copy(&file)? };

                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: Some(opts_bk),
                    typ: MmapFileMutType::Cow,
                })
            }
        }
    }
}

impl_sync_tests!("disk", DiskMmapFile, DiskMmapFileMut);

#[test]
fn test_close_with_truncate_on_empty_file() {
    let file = DiskMmapFileMut::create("disk_close_with_truncate_test.txt").unwrap();
    scopeguard::defer!(std::fs::remove_file("disk_close_with_truncate_test.txt").unwrap());
    file.close_with_truncate(10).unwrap();
    assert_eq!(
        10,
        File::open("disk_close_with_truncate_test.txt")
            .unwrap()
            .metadata()
            .unwrap()
            .len()
    );
}
