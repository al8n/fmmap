use crate::error::{Error, ErrorKind, Result};
#[cfg(feature = "nightly")]
use std::io;
// use std::ops::{Bound, RangeBounds};
use std::path::Path;

cfg_sync! {
    use std::fs::{File, OpenOptions};

    /// Sync directory
    pub fn sync_dir<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref();
        if !path.is_dir() {
            #[cfg(feature = "nightly")]
            return Err(Error::new(ErrorKind::IO, io::Error::from(io::ErrorKind::NotADirectory)));

            #[cfg(not(feature = "nightly"))]
            return Err(Error::from(ErrorKind::NotADirectory));
        }

        File::open(path)
            .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))?
            .sync_all()
            .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))
    }

    /// Sync parent
    pub fn sync_parent<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref().canonicalize().map_err(|e| Error::new(ErrorKind::IO, e))?;
        let path = path.parent().unwrap();
        if !path.is_dir() {
            #[cfg(feature = "nightly")]
            return Err(Error::new(ErrorKind::IO, io::Error::from(io::ErrorKind::NotADirectory)));
            #[cfg(not(feature = "nightly"))]
            return Err(Error::from(ErrorKind::NotADirectory));
        }
        File::open(path)
            .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))?
            .sync_all()
            .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))
    }

    /// Open a read-only file
    pub fn open_read_only_file<P: AsRef<Path>>(path: P) -> Result<File> {
        OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|e| Error::new(ErrorKind::IO, e))
    }

    /// Open an existing file in write mode, all writes will overwrite the original file
    pub fn open_exist_file<P: AsRef<Path>>(path: P) -> Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .append(false)
            .open(path)
            .map_err(|e| Error::new(ErrorKind::IO, e))
    }

    /// Open an existing file in write mode, all writes will append to the file
    pub fn open_exist_file_with_append<P: AsRef<Path>>(path: P) -> Result<File> {
        OpenOptions::new()
            .read(true)
            .append(true)
            .open(path)
            .map_err(|e| Error::new(ErrorKind::IO, e))
    }

    /// Open an existing file and truncate it
    pub fn open_file_with_truncate<P: AsRef<Path>>(path: P) -> Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(|e| Error::new(ErrorKind::IO, e))
    }

    /// Open or create a file
    pub fn open_or_create_file<P: AsRef<Path>>(path: P) -> Result<File> {
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| Error::new(ErrorKind::IO, e))
    }

    /// Create a new file
    pub fn create_file<P: AsRef<Path>>(path: P) -> Result<File> {
        OpenOptions::new()
            .create_new(true)
            .read(true)
            .append(true)
            .open(path)
            .map_err(|e| Error::new(ErrorKind::IO, e))
    }
}

cfg_async! {
    macro_rules! impl_async_file_utils {
        ($file: ident, $open_options: ident) => {
            /// Open a read-only file
            pub async fn open_read_only_file_async<P: AsRef<Path>>(path: P) -> Result<$file> {
                <$open_options>::new()
                    .read(true)
                    .open(path)
                    .await
                    .map_err(|e| Error::new(ErrorKind::IO, e))
            }

            /// Open an existing file in write mode, all writes will overwrite the original file
            pub async fn open_exist_file_async<P: AsRef<Path>>(path: P) -> Result<$file> {
                <$open_options>::new()
                    .read(true)
                    .write(true)
                    .append(false)
                    .open(path)
                    .await
                    .map_err(|e| Error::new(ErrorKind::IO, e))
            }

            /// Open an existing file in write mode, all writes will append to the file
            pub async fn open_exist_file_with_append_async<P: AsRef<Path>>(path: P) -> Result<$file> {
                <$open_options>::new()
                    .read(true)
                    .write(true)
                    .append(true)
                    .open(path)
                    .await
                    .map_err(|e| Error::new(ErrorKind::IO, e))
            }

            /// Open an existing file and truncate it
            pub async fn open_file_with_truncate_async<P: AsRef<Path>>(path: P) -> Result<$file> {
                <$open_options>::new()
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .open(path)
                    .await
                    .map_err(|e| Error::new(ErrorKind::IO, e))
            }

            /// Open or create a file
            pub async fn open_or_create_file_async<P: AsRef<Path>>(path: P) -> Result<$file> {
                <$open_options>::new()
                    .create(true)
                    .read(true)
                    .write(true)
                    .open(path)
                    .await
                    .map_err(|e| Error::new(ErrorKind::IO, e))
            }

            /// Create a new file
            pub async fn create_file_async<P: AsRef<Path>>(path: P) -> Result<$file> {
                <$open_options>::new()
                    .create_new(true)
                    .read(true)
                    .write(true)
                    .append(true)
                    .open(path)
                    .await
                    .map_err(|e| Error::new(ErrorKind::IO, e))
            }
        };
    }
}

cfg_smol! {
    /// file open utils for smol
    pub mod smol {
        use smol::fs::{File, OpenOptions};
        use crate::error::{Error, ErrorKind, Result};
        #[cfg(feature = "nightly")]
        use std::io;
        use std::path::Path;

        /// Sync directory
        pub async fn sync_dir_async<P: AsRef<Path>>(path: P) -> Result<()> {
            let path = path.as_ref();
            if !path.is_dir() {
                #[cfg(feature = "nightly")]
                return Err(Error::new(ErrorKind::IO, io::Error::from(io::ErrorKind::NotADirectory)));

                #[cfg(not(feature = "nightly"))]
                return Err(Error::from(ErrorKind::NotADirectory));
            }

            File::open(path)
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))?
                .sync_all()
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))
        }

        /// Sync parent directory
        pub async fn sync_parent_async<P: AsRef<Path>>(path: P) -> Result<()> {
            let path = path.as_ref().canonicalize().map_err(|e| Error::new(ErrorKind::IO, e))?;
            let path = path.parent().unwrap();
            if !path.is_dir() {
                #[cfg(feature = "nightly")]
                return Err(Error::new(ErrorKind::IO, io::Error::from(io::ErrorKind::NotADirectory)));

                #[cfg(not(feature = "nightly"))]
                return Err(Error::from(ErrorKind::NotADirectory));
            }

            File::open(path)
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))?
                .sync_all()
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))
        }

        impl_async_file_utils!(File, OpenOptions);
    }
}

cfg_tokio! {
    /// file open utils for tokio
    pub mod tokio {
        use tokio::fs::{File, OpenOptions};
        use crate::error::{Error, ErrorKind, Result};
        #[cfg(feature = "nightly")]
        use std::io;
        use std::path::Path;

        /// Sync directory
        pub async fn sync_dir_async<P: AsRef<Path>>(path: P) -> Result<()> {
            let path = path.as_ref();
            if !path.is_dir() {
                #[cfg(feature = "nightly")]
                return Err(Error::new(ErrorKind::IO, io::Error::from(io::ErrorKind::NotADirectory)));

                #[cfg(not(feature = "nightly"))]
                return Err(Error::from(ErrorKind::NotADirectory));
            }

            File::open(path)
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))?
                .sync_all()
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))
        }

        /// Sync parent directory
        pub async fn sync_parent_async<P: AsRef<Path>>(path: P) -> Result<()> {
            let path = path.as_ref().canonicalize().map_err(|e| Error::new(ErrorKind::IO, e))?;
            let path = path.parent().unwrap();
            if !path.is_dir() {
                #[cfg(feature = "nightly")]
                return Err(Error::new(ErrorKind::IO, io::Error::from(io::ErrorKind::NotADirectory)));

                #[cfg(not(feature = "nightly"))]
                return Err(Error::from(ErrorKind::NotADirectory));
            }

            File::open(path)
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))?
                .sync_all()
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))
        }

        impl_async_file_utils!(File, OpenOptions);
    }
}

cfg_async_std! {
    /// file open utils for async-std
    pub mod async_std {
        use async_std::fs::{File, OpenOptions};
        use crate::error::{Error, ErrorKind, Result};
        #[cfg(feature = "nightly")]
        use std::io;
        use async_std::path::Path;

        /// Sync directory
        pub async fn sync_dir_async<P: AsRef<Path>>(path: P) -> Result<()> {
            let path = path.as_ref();
            if !path.is_dir().await {
                #[cfg(feature = "nightly")]
                return Err(Error::new(ErrorKind::IO, io::Error::from(io::ErrorKind::NotADirectory)));

                #[cfg(not(feature = "nightly"))]
                return Err(Error::from(ErrorKind::NotADirectory));
            }

            File::open(path)
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))?
                .sync_all()
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))
        }

        /// Sync parent directory
        pub async fn sync_parent_async<P: AsRef<Path>>(path: P) -> Result<()> {
            let path = path.as_ref().canonicalize().await.map_err(|e| Error::new(ErrorKind::IO, e))?;
            let path = path.parent().unwrap();
            if !path.is_dir().await {
                #[cfg(feature = "nightly")]
                return Err(Error::new(ErrorKind::IO, io::Error::from(io::ErrorKind::NotADirectory)));

                #[cfg(not(feature = "nightly"))]
                return Err(Error::from(ErrorKind::NotADirectory));
            }

            File::open(path)
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))?
                .sync_all()
                .await
                .map_err(|e| Error::new_source_msg(ErrorKind::OpenFailed, path.to_string_lossy(), e))
        }

        impl_async_file_utils!(File, OpenOptions);
    }
}

// // TODO: enum_dispatch does not support impl as parameter
// #[allow(dead_code)]
// pub(crate) fn handle_range_unchecked(
//     range: impl RangeBounds<usize>,
//     upper_bound: usize,
// ) -> (usize, usize) {
//     let begin = match range.start_bound() {
//         Bound::Included(&n) => n,
//         Bound::Excluded(&n) => n + 1,
//         Bound::Unbounded => 0,
//     };
//
//     let end = match range.end_bound() {
//         Bound::Included(&n) => n.checked_add(1).expect("out of range"),
//         Bound::Excluded(&n) => n,
//         Bound::Unbounded => upper_bound,
//     };
//
//     assert!(
//         begin < end,
//         "range start must less than end: {:?} <= {:?}",
//         begin,
//         end,
//     );
//     assert!(
//         end <= upper_bound,
//         "range end out of bounds: {:?} <= {:?}",
//         end,
//         upper_bound,
//     );
//     (begin, end)
// }
//
// // TODO: enum_dispatch does not support impl as parameter
// #[allow(dead_code)]
// pub(crate) fn handle_range(
//     range: impl RangeBounds<usize>,
//     upper_bound: usize,
// ) -> Result<(usize, usize)> {
//     let begin = match range.start_bound() {
//         Bound::Included(&n) => n,
//         Bound::Excluded(&n) => n + 1,
//         Bound::Unbounded => 0,
//     };
//
//     let end = match range.end_bound() {
//         Bound::Included(&n) => n.checked_add(1),
//         Bound::Excluded(&n) => Some(n),
//         Bound::Unbounded => Some(upper_bound),
//     }
//     .ok_or(Error::OutOfBound(usize::MAX, upper_bound))?;
//
//     if begin >= end {
//         return Err(Error::InvalidBound(begin, end));
//     }
//
//     if end > upper_bound {
//         return Err(Error::OutOfBound(end, upper_bound));
//     }
//
//     Ok((begin, end))
// }
