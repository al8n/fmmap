#![cfg_attr(feature = "nightly", feature(is_symlink))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
#[macro_use]
extern crate thiserror;

#[macro_use]
extern crate enum_dispatch;

macro_rules! cfg_smol {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "smol-async")]
            #[cfg_attr(docsrs, doc(cfg(feature = "smol-async")))]
            $item
        )*
    }
}

macro_rules! cfg_tokio {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "tokio-async")]
            #[cfg_attr(docsrs, doc(cfg(feature = "tokio-async")))]
            $item
        )*
    }
}

macro_rules! cfg_sync {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "sync")]
            #[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
            $item
        )*
    }
}

macro_rules! cfg_windows {
    ($($item:item)*) => {
        $(
            #[cfg(windows)]
            #[cfg_attr(docsrs, doc(cfg(windows)))]
            $item
        )*
    }
}

macro_rules! cfg_unix {
    ($($item:item)*) => {
        $(
            #[cfg(unix)]
            #[cfg_attr(docsrs, doc(cfg(unix)))]
            $item
        )*
    }
}

mod mmap_file;
pub mod error;
mod metadata;
mod memory;
mod disk;
mod empty;
mod writer;
mod reader;
mod options;
mod utils;

cfg_sync!(
    pub use reader::MmapFileReader;
    pub use writer::MmapFileWriter;
    pub use mmap_file::{MmapFileExt, MmapFileMutExt};
);

cfg_tokio!(
    pub use reader::tokio_impl::AsyncMmapFileReader;
    pub use writer::tokio_impl::AsyncMmapFileWriter;
    pub use mmap_file::{AsyncMmapFileExt, AsyncMmapFileMutExt};
);

pub use metadata::{MetaData, MetaDataExt};

