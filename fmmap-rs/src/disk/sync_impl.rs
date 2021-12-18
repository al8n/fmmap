use std::fs::{File, remove_file};
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "linux"))]
use std::ptr::{drop_in_place, write};
use memmap2::{Mmap, MmapMut, MmapOptions};
use crate::{MetaData, MmapFileExt, MmapFileMutExt};
use crate::disk::{MmapFileMutType, remmap};
use crate::error::Error;
use crate::options::Options;
use crate::utils::{create_file, open_exist_file_with_append, open_or_create_file, open_read_only_file, sync_dir};

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
    pub fn open<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
        Self::open_in(path, None)
    }

    /// Open a readable memory map backed by a file with [`Options`]
    ///
    /// [`Options`]: structs.Options.html
    pub fn open_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
        Self::open_in(path, Some(opts))
    }

    /// Open a readable and executable memory map backed by a file
    pub fn open_exec<P: AsRef<Path>>(path: P,) -> Result<Self, Error> {
        Self::open_exec_in(path, None)
    }

    /// Open a readable and executable memory map backed by a file with [`Options`].
    ///
    /// [`Options`]: structs.Options.html
    pub fn open_exec_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
        Self::open_exec_in(path, Some(opts))
    }

    fn open_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
        let file = open_read_only_file(&path).map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;

        match opts  {
            None => {
                let mmap = unsafe {
                    Mmap::map(&file).map_err(|e| Error::MmapFailed(e.to_string()))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    exec: false
                })
            }
            Some(opts) => {
                let mmap = unsafe {
                    opts.mmap_opts.map(&file).map_err(|e| Error::MmapFailed(e.to_string()))?
                };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    exec: false
                })
            }
        }
    }

    fn open_exec_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
        let file = open_read_only_file(&path).map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;

        match opts  {
            None => {
                let mmap = unsafe {
                    MmapOptions::new().map_exec(&file).map_err(|e| Error::MmapFailed(e.to_string()))?
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
                    opts.mmap_opts.map_exec(&file).map_err(|e| Error::MmapFailed(e.to_string()))?
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
        // sync data
        self.flush()?;

        unsafe {
            // unmap
            drop_in_place(&mut self.mmap);

            // truncate
            self.file.set_len(max_sz).map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", self.path(), e)))?;

            // remap
            let mmap = remmap(self.path(), &self.file, self.opts.as_ref(), self.typ)?;

            write(&mut self.mmap, mmap);
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn truncate(&mut self, max_sz: u64) -> Result<(), Error> {
        // sync data
        self.flush()?;

        // truncate
        self.file.set_len(max_sz).map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", self.path(), e)))?;

        // remap
        self.mmap = remmap(self.path(), &self.file, self.opts.as_ref(), self.typ)?;

        Ok(())
    }

    fn remove(self) -> crate::error::Result<()> {
        let path = self.path;
        drop(self.mmap);
        self.file.set_len(0).map_err(Error::IO)?;
        drop(self.file);
        remove_file(path).map_err(Error::IO)
    }

    fn close_with_truncate(self, max_sz: i64) -> crate::error::Result<()> {
        self.flush()?;
        drop(self.mmap);
        if max_sz >= 0 {
            self.file.set_len(max_sz as u64).map_err(Error::IO)?;
            let parent = self.path.parent().unwrap();
            File::open(parent).map_err(Error::IO)?.sync_all().map_err(|e| Error::SyncDirFailed(e.to_string()))?
        }
        Ok(())
    }
}

impl DiskMmapFileMut {
    /// Create a new file and mmap this file
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::create_in(path, None)
    }

    /// Create a new file and mmap this file with [`Options`]
    ///
    /// [`Options`]: structs.Options.html
    pub fn create_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
         Self::create_in(path, Some(opts))
    }

    /// Open a file and mmap this file.
    /// The file will be open by [`File::open`].
    ///
    /// [`File::open`]: https://doc.rust-lang.org/std/fs/struct.File.html#method.open
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_in(path, None)
    }

    /// Open a file and mmap this file with [`Options`].
    /// The file will be open by [OpenOptions::open]
    ///
    /// [`Options`]: structs.Options.html
    /// [`OpenOptions::open`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.open
    pub fn open_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
        Self::open_in(path, Some(opts))
    }

    /// Open an existing file and mmap this file
    ///
    /// # Examples
    /// ```rust
    /// use fmmap::{MmapFileExt, MmapFileMutExt};
    /// use fmmap::raw::DiskMmapFileMut;
    /// use std::fs::{remove_file, File};
    /// use std::io::{Read, Write};
    /// use scopeguard::defer;
    ///
    /// // create a temp file
    /// let mut file = File::create("../scripts/disk_open_existing_test.txt").unwrap();
    /// defer!(remove_file("../scripts/disk_open_existing_test.txt").unwrap());
    /// file.write_all("some data...".as_bytes()).unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = DiskMmapFileMut::open_exist("../scripts/disk_open_existing_test.txt").unwrap();
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
    ///
    /// // reopen to check content
    /// let mut buf = vec![0; "some modified data...".len()];
    /// let mut file = File::open("../scripts/disk_open_existing_test.txt").unwrap();
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: structs.Options.html
    pub fn open_exist<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_exist_in(path, None)
    }

    /// Open an existing file and mmap this file with [`Options`]
    ///
    /// [`Options`]: structs.Options.html
    pub fn open_exist_with_options<P: AsRef<Path>>(path: P, opts: Options) -> Result<Self, Error> {
        Self::open_exist_in(path, Some(opts))
    }

    /// Open and mmap an existing file in copy-on-write mode.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fmmap::{MmapFileExt, MmapFileMutExt};
    /// use fmmap::raw::DiskMmapFileMut;
    /// use std::fs::{remove_file, File};
    /// use std::io::{Read, Write};
    /// use scopeguard::defer;
    ///
    /// // create a temp file
    /// let mut file = File::create("../scripts/disk_open_cow_test.txt").unwrap();
    /// defer!(remove_file("../scripts/disk_open_cow_test.txt").unwrap());
    /// file.write_all("some data...".as_bytes()).unwrap();
    /// drop(file);
    ///
    /// // mmap the file
    /// let mut file = DiskMmapFileMut::open_cow("../scripts/disk_open_cow_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice(), 0).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    ///
    /// // modify the file data
    /// file.truncate("some modified data...".len() as u64).unwrap();
    /// file.write_all("some modified data...".as_bytes(), 0).unwrap();
    /// file.flush().unwrap();
    ///
    /// // cow, change will only be seen in current caller
    /// assert_eq!(file.as_slice(), "some modified data...".as_bytes());
    /// drop(file);
    ///
    /// // reopen to check content, cow will not change the content.
    /// let mut file = File::open("../scripts/disk_open_cow_test.txt").unwrap();
    /// let mut buf = vec![0; "some data...".len()];
    /// file.read_exact(buf.as_mut_slice()).unwrap();
    /// assert_eq!(buf.as_slice(), "some data...".as_bytes());
    /// ```
    ///
    /// [`Options`]: structs.Options.html
    pub fn open_cow<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::open_cow_in(path, None)
    }

    /// Open and mmap an existing file in copy-on-write mode with [`Options`].
    /// Data written to the memory map will not be visible by other processes, and will not be carried through to the underlying file
    ///
    /// [`Options`]: structs.Options.html
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
    pub fn freeze(self) -> Result<DiskMmapFile, Error> {
        Ok(DiskMmapFile {
            mmap: self.mmap.make_read_only().map_err(Error::IO)?,
            file: self.file,
            path: self.path,
            exec: false
        })
    }

    /// Transition the memory map to be readable and executable.
    /// If the memory map is file-backed, the file must have been opened with execute permissions.
    ///
    /// # Errors
    /// This method returns an error when the underlying system call fails,
    /// which can happen for a variety of reasons,
    /// such as when the file has not been opened with execute permissions
    pub fn freeze_exec(self) -> Result<DiskMmapFile, Error> {
        Ok(DiskMmapFile {
            mmap: self.mmap.make_exec().map_err(Error::IO)?,
            file: self.file,
            path: self.path,
            exec: true,
        })
    }

    fn create_in<P: AsRef<Path>>(path: P, opts: Option<Options>) -> Result<Self, Error> {
        let file = create_file(&path)
            .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;

        match opts {
            None => {
                let mmap = unsafe { MmapMut::map_mut(&file).map_err(|e| Error::MmapFailed(e.to_string()))? };

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
                    file.set_len(opts.max_size).map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", path.as_ref(), e)))?;
                    let parent = path.as_ref().parent().unwrap();
                    sync_dir(parent)?;
                }

                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe { opts.mmap_opts.map_mut(&file).map_err(|e| Error::MmapFailed(e.to_string()))? };

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
                let file = open_or_create_file(&path).map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;
                let mmap = unsafe { MmapMut::map_mut(&file).map_err(|e| Error::MmapFailed(e.to_string()))? };
                Ok(Self {
                    mmap,
                    file,
                    path: path.as_ref().to_path_buf(),
                    opts: None,
                    typ: MmapFileMutType::Normal,
                })
            }
            Some(opts) => {
                let file = opts.file_opts.open(&path).map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;
                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe {
                    opts.mmap_opts.map_mut(&file).map_err(|e| Error::MmapFailed(e.to_string()))?
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
        let file = open_exist_file_with_append(&path)
            .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;

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
                    file.set_len(opts.max_size).map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", path.as_ref(), e)))?;
                    let parent = path.as_ref().parent().unwrap();
                    sync_dir(parent)?;
                }
                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe {
                    opts.mmap_opts.map_mut(&file)? };

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
        let file = open_exist_file_with_append(&path)
            .map_err(|e| Error::OpenFailed(format!("path: {:?}, err: {:?}", path.as_ref(), e)))?;

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
                let meta = file.metadata()?;
                let file_sz = meta.len();
                if file_sz == 0 && opts.max_size > 0 {
                    file.set_len(opts.max_size).map_err(|e| Error::TruncationFailed(format!("path: {:?}, err: {}", path.as_ref(), e)))?;
                    let parent = path.as_ref().parent().unwrap();
                    sync_dir(parent)?;
                }
                let opts_bk = opts.mmap_opts.clone();
                let mmap = unsafe {
                    opts.mmap_opts.map_copy(&file)? };

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