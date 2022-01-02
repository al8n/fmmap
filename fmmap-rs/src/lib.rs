//! <div align="center">
//! <h1>fmmap</h1>
//! </div>
//! <div align="center">
//!
//! A flexible and convenient high-level mmap for zero-copy file I/O.
//!
//! English | [简体中文](README-zh_CN.md)
//!
//! [<img alt="github" src="https://img.shields.io/badge/GITHUB-fmmap-8da0cb?style=for-the-badge&logo=Github" height="22">][Github-url]
//! [<img alt="Build" src="https://img.shields.io/github/workflow/status/al8n/vela/CI/main?logo=Github-Actions&style=for-the-badge" height="22">][CI-url]
//! [<img alt="codecov" src="https://img.shields.io/codecov/c/gh/al8n/fmmap?style=for-the-badge&token=A5KY75ACD8&logo=codecov" height="22">][codecov-url]
//!
//! [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-fmmap-66c2a5?style=for-the-badge&labelColor=555555&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">][doc-url]
//! [<img alt="crates.io" src="https://img.shields.io/crates/v/fmmap?style=for-the-badge&logo=data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iaXNvLTg4NTktMSI/Pg0KPCEtLSBHZW5lcmF0b3I6IEFkb2JlIElsbHVzdHJhdG9yIDE5LjAuMCwgU1ZHIEV4cG9ydCBQbHVnLUluIC4gU1ZHIFZlcnNpb246IDYuMDAgQnVpbGQgMCkgIC0tPg0KPHN2ZyB2ZXJzaW9uPSIxLjEiIGlkPSJMYXllcl8xIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHhtbG5zOnhsaW5rPSJodHRwOi8vd3d3LnczLm9yZy8xOTk5L3hsaW5rIiB4PSIwcHgiIHk9IjBweCINCgkgdmlld0JveD0iMCAwIDUxMiA1MTIiIHhtbDpzcGFjZT0icHJlc2VydmUiPg0KPGc+DQoJPGc+DQoJCTxwYXRoIGQ9Ik0yNTYsMEwzMS41MjgsMTEyLjIzNnYyODcuNTI4TDI1Niw1MTJsMjI0LjQ3Mi0xMTIuMjM2VjExMi4yMzZMMjU2LDB6IE0yMzQuMjc3LDQ1Mi41NjRMNzQuOTc0LDM3Mi45MTNWMTYwLjgxDQoJCQlsMTU5LjMwMyw3OS42NTFWNDUyLjU2NHogTTEwMS44MjYsMTI1LjY2MkwyNTYsNDguNTc2bDE1NC4xNzQsNzcuMDg3TDI1NiwyMDIuNzQ5TDEwMS44MjYsMTI1LjY2MnogTTQzNy4wMjYsMzcyLjkxMw0KCQkJbC0xNTkuMzAzLDc5LjY1MVYyNDAuNDYxbDE1OS4zMDMtNzkuNjUxVjM3Mi45MTN6IiBmaWxsPSIjRkZGIi8+DQoJPC9nPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPGc+DQo8L2c+DQo8Zz4NCjwvZz4NCjxnPg0KPC9nPg0KPC9zdmc+DQo=" height="22">][crates-url]
//! [<img alt="rustc" src="https://img.shields.io/badge/MSRV-1.56.0-fc8d62.svg?style=for-the-badge&logo=Rust" height="22">][rustc-url]
//!
//! [<img alt="license-apache" src="https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=for-the-badge&logo=Apache" height="22">][license-apache-url]
//! [<img alt="license-mit" src="https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge&fontColor=white&logoColor=f5c076&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIGhlaWdodD0iMzZweCIgdmlld0JveD0iMCAwIDI0IDI0IiB3aWR0aD0iMzZweCIgZmlsbD0iI2Y1YzA3NiI+PHBhdGggZD0iTTAgMGgyNHYyNEgwVjB6IiBmaWxsPSJub25lIi8+PHBhdGggZD0iTTEwLjA4IDEwLjg2Yy4wNS0uMzMuMTYtLjYyLjMtLjg3cy4zNC0uNDYuNTktLjYyYy4yNC0uMTUuNTQtLjIyLjkxLS4yMy4yMy4wMS40NC4wNS42My4xMy4yLjA5LjM4LjIxLjUyLjM2cy4yNS4zMy4zNC41My4xMy40Mi4xNC42NGgxLjc5Yy0uMDItLjQ3LS4xMS0uOS0uMjgtMS4yOXMtLjQtLjczLS43LTEuMDEtLjY2LS41LTEuMDgtLjY2LS44OC0uMjMtMS4zOS0uMjNjLS42NSAwLTEuMjIuMTEtMS43LjM0cy0uODguNTMtMS4yLjkyLS41Ni44NC0uNzEgMS4zNlM4IDExLjI5IDggMTEuODd2LjI3YzAgLjU4LjA4IDEuMTIuMjMgMS42NHMuMzkuOTcuNzEgMS4zNS43Mi42OSAxLjIuOTFjLjQ4LjIyIDEuMDUuMzQgMS43LjM0LjQ3IDAgLjkxLS4wOCAxLjMyLS4yM3MuNzctLjM2IDEuMDgtLjYzLjU2LS41OC43NC0uOTQuMjktLjc0LjMtMS4xNWgtMS43OWMtLjAxLjIxLS4wNi40LS4xNS41OHMtLjIxLjMzLS4zNi40Ni0uMzIuMjMtLjUyLjNjLS4xOS4wNy0uMzkuMDktLjYuMS0uMzYtLjAxLS42Ni0uMDgtLjg5LS4yMy0uMjUtLjE2LS40NS0uMzctLjU5LS42MnMtLjI1LS41NS0uMy0uODgtLjA4LS42Ny0uMDgtMXYtLjI3YzAtLjM1LjAzLS42OC4wOC0xLjAxek0xMiAyQzYuNDggMiAyIDYuNDggMiAxMnM0LjQ4IDEwIDEwIDEwIDEwLTQuNDggMTAtMTBTMTcuNTIgMiAxMiAyem0wIDE4Yy00LjQxIDAtOC0zLjU5LTgtOHMzLjU5LTggOC04IDggMy41OSA4IDgtMy41OSA4LTggOHoiLz48L3N2Zz4=" height="22">][license-mit-url]
//!
//! </div>
//!
//! ## License
//!
//! <sup>
//! Licensed under either of <a href="https://opensource.org/licenses/Apache-2.0">Apache License, Version
//! 2.0</a> or <a href="https://opensource.org/licenses/MIT">MIT license</a> at your option.
//! </sup>
//!
//! <br>
//!
//! <sub>
//! Unless you explicitly state otherwise, any contribution intentionally submitted
//! for inclusion in this project by you, as defined in the Apache-2.0 license,
//! shall be dual licensed as above, without any additional terms or conditions.
//! </sub>
//!
//!
//! [Github-url]: https://github.com/al8n/fmmap/
//! [CI-url]: https://github.com/al8n/fmmap/actions/workflows/ci.yml
//! [doc-url]: https://docs.rs/fmmap
//! [crates-url]: https://crates.io/crates/fmmap
//! [codecov-url]: https://app.codecov.io/gh/al8n/fmmap/
//! [license-url]: https://opensource.org/licenses/Apache-2.0
//! [rustc-url]: https://github.com/rust-lang/rust/blob/master/RELEASES.md
//! [license-apache-url]: https://opensource.org/licenses/Apache-2.0
//! [license-mit-url]: https://opensource.org/licenses/MIT
//!
#![cfg_attr(feature = "nightly", feature(io_error_more))]
#![cfg_attr(all(feature = "nightly", windows), feature(windows_by_handle))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
#![allow(
    rustdoc::broken_intra_doc_links,
    unused_macros,
    clippy::len_without_is_empty,
    clippy::upper_case_acronyms
)]
#![deny(missing_docs)]
#[doc = include_str!("../README.md")]
#[macro_use]
extern crate enum_dispatch;

macro_rules! cfg_async_std {
    ($($item:item)*) => {
        $(
            #[cfg(all(feature = "async-std", feature = "async-trait"))]
            #[cfg_attr(docsrs, doc(cfg(all(feature = "async-std", feature = "async-trait"))))]
            $item
        )*
    }
}

macro_rules! cfg_smol {
    ($($item:item)*) => {
        $(
            #[cfg(all(feature = "smol", feature = "async-trait"))]
            #[cfg_attr(docsrs, doc(cfg(all(feature = "smol", feature = "async-trait"))))]
            $item
        )*
    }
}

macro_rules! cfg_tokio {
    ($($item:item)*) => {
        $(
            #[cfg(all(feature = "tokio", feature = "async-trait"))]
            #[cfg_attr(docsrs, doc(cfg(all(feature = "tokio", feature = "async-trait"))))]
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

macro_rules! cfg_async {
    ($($item:item)*) => {
        $(
            #[cfg(all(any(feature = "smol", feature = "async-std", feature = "tokio"), feature = "async-trait"))]
            #[cfg_attr(docsrs, doc(cfg(all(any(feature = "smol", feature = "async-std", feature = "tokio"), feature = "async-trait"))))]
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

macro_rules! noop_flush {
    () => {
        #[inline(always)]
        fn flush(&self) -> crate::error::Result<()> {
            Ok(())
        }

        #[inline(always)]
        fn flush_async(&self) -> crate::error::Result<()> {
            Ok(())
        }

        #[inline(always)]
        fn flush_range(&self, _offset: usize, _len: usize) -> crate::error::Result<()> {
            Ok(())
        }

        #[inline(always)]
        fn flush_async_range(&self, _offset: usize, _len: usize) -> crate::error::Result<()> {
            Ok(())
        }
    };
}

macro_rules! noop_file_lock {
    () => {
        #[inline]
        fn lock_exclusive(&self) -> crate::error::Result<()> {
            Ok(())
        }

        #[inline]
        fn lock_shared(&self) -> crate::error::Result<()> {
            Ok(())
        }

        #[inline]
        fn try_lock_exclusive(&self) -> crate::error::Result<()> {
            Ok(())
        }

        #[inline]
        fn try_lock_shared(&self) -> crate::error::Result<()> {
            Ok(())
        }

        #[inline]
        fn unlock(&self) -> crate::error::Result<()> {
            Ok(())
        }
    };
}

mod disk;
mod empty;
/// Errors in this crate
pub mod error;
mod memory;
mod metadata;
mod mmap_file;
#[allow(dead_code)]
mod options;
mod reader;
#[cfg(test)]
pub mod tests;
/// File I/O utils function
pub mod utils;
mod writer;

cfg_sync!(
    /// std based mmap file
    pub mod sync {
        pub use crate::mmap_file::{MmapFile, MmapFileExt, MmapFileMut, MmapFileMutExt};
        pub use crate::options::Options;
        pub use crate::reader::{MmapFileReader, MmapFileReaderExt};
        pub use crate::writer::{MmapFileWriter, MmapFileWriterExt};
    }

    pub use reader::{MmapFileReader, MmapFileReaderExt};
    pub use writer::{MmapFileWriter, MmapFileWriterExt};
    pub use mmap_file::{MmapFileExt, MmapFileMutExt, MmapFile, MmapFileMut};
    pub use options::Options;
);

cfg_async!(
    #[macro_use]
    extern crate async_trait;
);

cfg_async_std!(
    /// async_std based mmap file
    pub mod async_std {
        pub use crate::mmap_file::async_std_impl::{
            AsyncMmapFile, AsyncMmapFileExt, AsyncMmapFileMut, AsyncMmapFileMutExt,
        };
        pub use crate::options::async_std_impl::AsyncOptions;
        pub use crate::reader::async_std_impl::AsyncMmapFileReader;
        pub use crate::writer::async_std_impl::AsyncMmapFileWriter;
    }
);

cfg_smol!(
    /// smol based mmap file
    pub mod smol {
        pub use crate::mmap_file::smol_impl::{
            AsyncMmapFile, AsyncMmapFileExt, AsyncMmapFileMut, AsyncMmapFileMutExt,
        };
        pub use crate::options::smol_impl::AsyncOptions;
        pub use crate::reader::smol_impl::AsyncMmapFileReader;
        pub use crate::writer::smol_impl::AsyncMmapFileWriter;
    }
);

cfg_tokio!(
    /// tokio based mmap file
    pub mod tokio {
        pub use crate::mmap_file::tokio_impl::{
            AsyncMmapFile, AsyncMmapFileExt, AsyncMmapFileMut, AsyncMmapFileMutExt,
        };
        pub use crate::options::tokio_impl::AsyncOptions;
        pub use crate::reader::tokio_impl::AsyncMmapFileReader;
        pub use crate::writer::tokio_impl::AsyncMmapFileWriter;
    }
);

pub use metadata::{MetaData, MetaDataExt};

/// Components of mmap file.
pub mod raw {
    cfg_sync!(
        /// std based raw mmap file
        ///
        /// Inner components of [`MmapFile`], [`MmapFileMut`]
        ///
        /// [`MmapFile`]: struct.MmapFile.html
        /// [`MmapFileMut`]: struct.MmapFileMut.html
        pub mod sync {
            pub use crate::disk::{DiskMmapFile, DiskMmapFileMut};
            pub use crate::memory::{MemoryMmapFile, MemoryMmapFileMut};
        }
        pub use crate::disk::{DiskMmapFile, DiskMmapFileMut};
        pub use crate::memory::{MemoryMmapFile, MemoryMmapFileMut};
    );

    cfg_async_std!(
        /// async_std based raw mmap file
        ///
        /// Inner components of [`AsyncMmapFile`], [`AsyncMmapFileMut`]
        ///
        /// [`AsyncMmapFile`]: async_std/struct.AsyncMmapFile.html
        /// [`AsyncMmapFileMut`]: async_std/struct.AsyncMmapFileMut.html
        pub mod async_std {
            pub use crate::disk::async_std_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};
            pub use crate::memory::async_std_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut};
        }
    );

    cfg_smol!(
        /// smol based raw mmap file
        ///
        /// Inner components of [`AsyncMmapFile`], [`AsyncMmapFileMut`]
        ///
        /// [`AsyncMmapFile`]: async_std/struct.AsyncMmapFile.html
        /// [`AsyncMmapFileMut`]: async_std/struct.AsyncMmapFileMut.html
        pub mod smol {
            pub use crate::disk::smol_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};
            pub use crate::memory::smol_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut};
        }
    );

    cfg_tokio!(
        /// tokio based raw mmap file
        ///
        /// Inner components of [`AsyncMmapFile`], [`AsyncMmapFileMut`]
        ///
        /// [`AsyncMmapFile`]: tokio/struct.AsyncMmapFile.html
        /// [`AsyncMmapFileMut`]: tokio/struct.AsyncMmapFileMut.html
        pub mod tokio {
            pub use crate::disk::tokio_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut};
            pub use crate::memory::tokio_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut};
        }
    );
}
