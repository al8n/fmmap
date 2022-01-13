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
//! [<img alt="Build" src="https://img.shields.io/github/workflow/status/al8n/fmmap/rust/main?logo=Github-Actions&style=for-the-badge" height="22">][CI-url]
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
//! ## Design
//! The design of this crate is inspired by Dgraph's mmap file implementation in [Stretto](https://github.com/dgraph-io/stretto).
//!
//! All of file-backed memory map has the potential for Undefined Behavior (UB) if the underlying file is subsequently modified (e.g. the file is deleted by another process), in or out of process, this crate tries to avoid this situation by provide file lock APIs.
//!
//! This crate supports std and popular async runtime(tokio, async-std, smol), and thanks to `macro` in Rust, it is super easy to support any new async runtime. For details, please see the implementation for tokio, async-std, smol of the source code.
//!
//! ## Features
//! - [x] dozens of file I/O util functions
//! - [x] file-backed memory maps
//! - [x] synchronous and asynchronous flushing
//! - [x] copy-on-write memory maps
//! - [x] read-only memory maps
//! - [x] stack support (`MAP_STACK` on unix)
//! - [x] executable memory maps
//! - [x] file locks.
//! - [x] [tokio][tokio]
//! - [x] [smol][smol]
//! - [x] [async-std][async-std]
//!
//! ## Installation
//! - std
//! ```toml
//! [dependencies]
//! fmmap = 0.2
//! ```
//!
//! - [tokio][tokio]
//! ```toml
//! [dependencies]
//! fmmap = { version = "0.2", features = ["tokio-async"] }
//! ```
//!
//! - [async-std][async-std]
//! ```toml
//! [dependencies]
//! fmmap = { version = "0.2", features = ["std-async"] }
//! ```
//!
//! - [smol][smol]
//! ```toml
//! [dependencies]
//! fmmap = { version = "0.2", features = ["smol-async"] }
//! ```
//!
//! ## Examples
//! This crate is 100% documented, see [documents][doc-url] for examples.
//!
//! ## TODO
//! - [ ] add benchmarks
//!
//! #### License
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
//! [CI-url]: https://github.com/al8n/fmmap/actions/workflows/rust.yml
//! [doc-url]: https://docs.rs/fmmap
//! [crates-url]: https://crates.io/crates/fmmap
//! [codecov-url]: https://app.codecov.io/gh/al8n/fmmap/
//! [license-url]: https://opensource.org/licenses/Apache-2.0
//! [rustc-url]: https://github.com/rust-lang/rust/blob/master/RELEASES.md
//! [license-apache-url]: https://opensource.org/licenses/Apache-2.0
//! [license-mit-url]: https://opensource.org/licenses/MIT
//! [tokio]: https://crates.io/crates/tokio
//! [smol]: https://crates.io/crates/smol
//! [async-std]: https://crates.io/crates/async-std
//!
#![cfg_attr(feature = "nightly", feature(io_error_more))]
#![cfg_attr(all(feature = "nightly", windows), feature(windows_by_handle))]
#![cfg_attr(all(feature = "nightly", unix), feature(is_symlink))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
#![doc(html_root_url = "https://docs.rs/fmmap/0.2.3")]
#![allow(
    rustdoc::broken_intra_doc_links,
    unused_macros,
    clippy::len_without_is_empty,
    clippy::upper_case_acronyms
)]
#![deny(missing_docs)]
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

/// `#[cfg(windows)]`
#[macro_export]
macro_rules! cfg_windows {
    ($($item:item)*) => {
        $(
            #[cfg(windows)]
            #[cfg_attr(docsrs, doc(cfg(windows)))]
            $item
        )*
    }
}

/// `#[cfg(unix)]`
#[macro_export]
macro_rules! cfg_unix {
    ($($item:item)*) => {
        $(
            #[cfg(unix)]
            #[cfg_attr(docsrs, doc(cfg(unix)))]
            $item
        )*
    }
}

/// `#[cfg(test)]`
#[macro_export]
macro_rules! cfg_test {
    ($($item:item)*) => {
        $(
            #[cfg(test)]
            #[cfg_attr(docsrs, doc(cfg(test)))]
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

cfg_sync! {
    macro_rules! impl_sync_tests {
        ($filename_prefix: literal, $mmap_file: ident, $mmap_file_mut: ident) => {
            #[cfg(test)]
            mod test {
                use super::*;
                use scopeguard::defer;
                use std::fs::File;
                use crate::MetaDataExt;

                #[test]
                fn test_flush() {
                    let path = concat!($filename_prefix, "_flush.txt");
                    let mut file1 = <$mmap_file_mut>::create_with_options(path, Options::new().max_size(100)).unwrap();
                    defer!(std::fs::remove_file(path).unwrap(););
                    file1.write_all(vec![1; 100].as_slice(), 0).unwrap();
                    file1.flush_range(0, 10).unwrap();
                    file1.flush_async_range(11, 20).unwrap();
                    file1.flush_async().unwrap();
                }

                #[test]
                fn test_lock_shared() {
                    let path = concat!($filename_prefix, "_lock_shared.txt");
                    let file1 = <$mmap_file_mut>::open(path).unwrap();
                    let file2 = <$mmap_file_mut>::open(path).unwrap();
                    let file3 = <$mmap_file>::open(path).unwrap();
                    defer!(std::fs::remove_file(path).unwrap());

                    // Concurrent shared access is OK, but not shared and exclusive.
                    file1.lock_shared().unwrap();
                    file2.lock_shared().unwrap();
                    assert!(file3.try_lock_exclusive().is_err());
                    file1.unlock().unwrap();
                    assert!(file3.try_lock_exclusive().is_err());

                    // Once all shared file locks are dropped, an exclusive lock may be created;
                    file2.unlock().unwrap();
                    file3.lock_exclusive().unwrap();
                }

                #[test]
                fn test_lock_exclusive() {
                    let path = concat!($filename_prefix, "_lock_exclusive.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let file1 = <$mmap_file_mut>::open(path).unwrap();
                    let file2 = <$mmap_file>::open(path).unwrap();

                    // No other access is possible once an exclusive lock is created.
                    file1.lock_exclusive().unwrap();
                    assert!(file2.try_lock_exclusive().is_err());
                    assert!(file2.try_lock_shared().is_err());

                    // Once the exclusive lock is dropped, the second file is able to create a lock.
                    file1.unlock().unwrap();
                    file2.lock_exclusive().unwrap();
                }

                #[test]
                fn test_lock_cleanup() {
                    let path = concat!($filename_prefix, "_lock_cleanup.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let file1 = <$mmap_file_mut>::open(path).unwrap();
                    let file2 = <$mmap_file>::open(path).unwrap();

                    // No other access is possible once an exclusive lock is created.
                    file1.lock_exclusive().unwrap();
                    assert!(file2.try_lock_exclusive().is_err());
                    assert!(file2.try_lock_shared().is_err());

                    // Drop file1; the lock should be released.
                    drop(file1);
                    file2.lock_shared().unwrap();
                }

                #[test]
                fn test_open() {
                    let path = concat!($filename_prefix, "_open_test.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).unwrap();

                    file.truncate(12).unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);
                    // mmap the file
                    let file = <$mmap_file>::open(path).unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[test]
                fn test_open_with_options() {
                    let path = concat!($filename_prefix, "_open_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).unwrap();
                    file.truncate(23).unwrap();
                    file.write_all("sanity text".as_bytes(), 0).unwrap();
                    file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let opts = Options::new()
                        // mmap content after the sanity text
                        .offset("sanity text".as_bytes().len() as u64);
                    // mmap the file
                    let file = <$mmap_file>::open_with_options(path, opts).unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[test]
                fn test_open_exec() {
                    let path = concat!($filename_prefix, "_open_exec.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).unwrap();
                    file.truncate(12).unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);
                    // mmap the file
                    let file = <$mmap_file>::open_exec(path).unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[test]
                fn test_open_exec_with_options() {
                    let path = concat!($filename_prefix, "_open_exec_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).unwrap();
                    file.truncate(23).unwrap();
                    file.write_all("sanity text".as_bytes(), 0).unwrap();
                    file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let opts = Options::new()
                        // mmap content after the sanity text
                        .offset("sanity text".as_bytes().len() as u64);
                    // mmap the file
                    let file = <$mmap_file>::open_exec_with_options(path, opts).unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[test]
                fn test_remove() {
                    let path = concat!($filename_prefix, "_remove.txt");
                    let mut file = <$mmap_file_mut>::create(path).unwrap();

                    file.truncate(12).unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    file.remove().unwrap();

                    let err = File::open(path);
                    assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
                }

                #[test]
                fn test_close_with_truncate() {
                    let path = concat!($filename_prefix, "_close_with_truncate.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).unwrap();

                    file.truncate(100).unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    file.close_with_truncate(50).unwrap();

                    let file = <$mmap_file_mut>::open(path).unwrap();
                    let meta = file.metadata().unwrap();
                    assert_eq!(meta.len(), 50);
                }

                #[test]
                fn test_create() {
                    let path = concat!($filename_prefix, "_create.txt");
                    let mut file = <$mmap_file_mut>::create(path).unwrap();
                    defer!(std::fs::remove_file(path).unwrap(););
                    file.truncate(12).unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                }

                #[test]
                fn test_create_with_options() {
                    let path = concat!($filename_prefix, "_create_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let opts = Options::new()
                        // truncate to 100
                        .max_size(100);
                    let mut file = <$mmap_file_mut>::create_with_options(path, opts).unwrap();

                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                }

                #[test]
                fn test_open_mut() {
                    let path = concat!($filename_prefix, "_open_mut.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).unwrap();

                    file.truncate(12).unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let mut file = <$mmap_file_mut>::open(path).unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.truncate("some modified data...".len() as u64).unwrap();
                    file.write_all("some modified data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // reopen to check content
                    let mut buf = vec![0; "some modified data...".len()];
                    let file = <$mmap_file_mut>::open(path).unwrap();
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                }

                #[test]
                fn test_open_mut_with_options() {
                    let path = concat!($filename_prefix, "_open_mut_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).unwrap();
                    file.truncate(23).unwrap();
                    file.write_all("sanity text".as_bytes(), 0).unwrap();
                    file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let opts = Options::new()
                        // allow read
                        .read(true)
                        // allow write
                        .write(true)
                        // allow append
                        .append(true)
                        // truncate to 100
                        .max_size(100)
                        // mmap content after the sanity text
                        .offset("sanity text".as_bytes().len() as u64);
                    let mut file = <$mmap_file_mut>::open_with_options(path, opts).unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.truncate(("some modified data...".len() + "sanity text".len()) as u64).unwrap();
                    file.write_all("some modified data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // reopen to check content
                    let mut buf = vec![0; "some modified data...".len()];
                    let file = <$mmap_file_mut>::open(path).unwrap();
                    // skip the sanity text
                    file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
                    assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                }

                #[test]
                fn test_open_exist() {
                    let path = concat!($filename_prefix, "_open_exist.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    // create a temp file
                    let mut file = <$mmap_file_mut>::create(path).unwrap();
                    file.truncate(12).unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let mut file = <$mmap_file_mut>::open_exist(path).unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.truncate("some modified data...".len() as u64).unwrap();
                    file.write_all("some modified data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);


                    // reopen to check content
                    let mut buf = vec![0; "some modified data...".len()];
                    let file = <$mmap_file_mut>::open(path).unwrap();
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                }

                #[test]
                fn test_open_exist_with_options() {
                    let path = concat!($filename_prefix, "_open_exist_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    // create a temp file
                    let mut file = <$mmap_file_mut>::create(path).unwrap();
                    file.truncate(23).unwrap();
                    file.write_all("sanity text".as_bytes(), 0).unwrap();
                    file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let opts = Options::new()
                        // truncate to 100
                        .max_size(100)
                        // mmap content after the sanity text
                        .offset("sanity text".as_bytes().len() as u64);

                    let mut file = <$mmap_file_mut>::open_exist_with_options(path, opts).unwrap();

                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.truncate(("some modified data...".len() + "sanity text".len()) as u64).unwrap();
                    file.write_all("some modified data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();

                    // reopen to check content, cow will not change the content.
                    let file = <$mmap_file_mut>::open(path).unwrap();
                    let mut buf = vec![0; "some modified data...".len()];
                    // skip the sanity text
                    file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
                    assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                }

                #[test]
                fn test_open_cow() {
                    let path = concat!($filename_prefix, "_open_cow.txt");
                    defer!(std::fs::remove_file(path).unwrap());

                    // create a temp file
                    let mut file = <$mmap_file_mut>::create(path).unwrap();

                    file.truncate(12).unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let mut file = <$mmap_file_mut>::open_cow(path).unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.write_all("some data!!!".as_bytes(), 0).unwrap();
                    file.flush().unwrap();

                    // cow, change will only be seen in current caller
                    assert_eq!(file.as_slice(), "some data!!!".as_bytes());
                    drop(file);

                    // reopen to check content, cow will not change the content.
                    let file = <$mmap_file_mut>::open(path).unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[test]
                fn test_open_cow_with_options() {
                    let path = concat!($filename_prefix, "_open_cow_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());

                    // create a temp file
                    let mut file = <$mmap_file_mut>::create(path).unwrap();
                    file.truncate(23).unwrap();
                    file.write_all("sanity text".as_bytes(), 0).unwrap();
                    file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let opts = Options::new()
                        // mmap content after the sanity text
                        .offset("sanity text".as_bytes().len() as u64);

                    let mut file = <$mmap_file_mut>::open_cow_with_options(path, opts).unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.write_all("some data!!!".as_bytes(), 0).unwrap();
                    file.flush().unwrap();

                    // cow, change will only be seen in current caller
                    assert_eq!(file.as_slice(), "some data!!!".as_bytes());
                    drop(file);

                    // reopen to check content, cow will not change the content.
                    let file = <$mmap_file_mut>::open(path).unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    // skip the sanity text
                    file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[test]
                fn test_freeze() {
                    let path = concat!($filename_prefix, "_freeze.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).unwrap();
                    file.truncate(12).unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    // freeze
                    file.freeze().unwrap();
                }

                #[test]
                fn test_freeze_exec() {
                    let path = concat!($filename_prefix, "_freeze_exec.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).unwrap();
                    file.truncate(12).unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    // freeze_exec
                    file.freeze_exec().unwrap();
                }
            }
        };
    }

}

cfg_async! {
    macro_rules! impl_async_tests {
        ($filename_prefix: literal, $runtime: meta, $path_str: ident, $mmap_file: ident, $mmap_file_mut: ident) => {
            #[cfg(test)]
            mod tests {
                use super::*;
                use scopeguard::defer;
                use $path_str::fs::File;
                use crate::MetaDataExt;

                #[$runtime]
                async fn test_flush() {
                    let path = concat!($filename_prefix, "_flush.txt");
                    let mut file1 = <$mmap_file_mut>::create_with_options(path, AsyncOptions::new().max_size(100)).await.unwrap();
                    defer!(std::fs::remove_file(path).unwrap(););
                    file1.write_all(vec![1; 100].as_slice(), 0).unwrap();
                    file1.flush_range(0, 10).unwrap();
                    file1.flush_async_range(11, 20).unwrap();
                    file1.flush_async().unwrap();
                }

                #[$runtime]
                async fn test_lock_shared() {
                    let path = concat!($filename_prefix, "_lock_shared.txt");
                    let file1 = <$mmap_file_mut>::open(path).await.unwrap();
                    let file2 = <$mmap_file_mut>::open(path).await.unwrap();
                    let file3 = <$mmap_file>::open(path).await.unwrap();
                    defer!(std::fs::remove_file(path).unwrap());

                    // Concurrent shared access is OK, but not shared and exclusive.
                    file1.lock_shared().unwrap();
                    file2.lock_shared().unwrap();
                    assert!(file3.try_lock_exclusive().is_err());
                    file1.unlock().unwrap();
                    assert!(file3.try_lock_exclusive().is_err());

                    // Once all shared file locks are dropped, an exclusive lock may be created;
                    file2.unlock().unwrap();
                    file3.lock_exclusive().unwrap();
                }

                #[$runtime]
                async fn test_lock_exclusive() {
                    let path = concat!($filename_prefix, "_lock_exclusive.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let file1 = <$mmap_file_mut>::open(path).await.unwrap();
                    let file2 = <$mmap_file>::open(path).await.unwrap();

                    // No other access is possible once an exclusive lock is created.
                    file1.lock_exclusive().unwrap();
                    assert!(file2.try_lock_exclusive().is_err());
                    assert!(file2.try_lock_shared().is_err());

                    // Once the exclusive lock is dropped, the second file is able to create a lock.
                    file1.unlock().unwrap();
                    file2.lock_exclusive().unwrap();
                }

                #[$runtime]
                async fn test_lock_cleanup() {
                    let path = concat!($filename_prefix, "_lock_cleanup.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let file1 = <$mmap_file_mut>::open(path).await.unwrap();
                    let file2 = <$mmap_file>::open(path).await.unwrap();

                    // No other access is possible once an exclusive lock is created.
                    file1.lock_exclusive().unwrap();
                    assert!(file2.try_lock_exclusive().is_err());
                    assert!(file2.try_lock_shared().is_err());

                    // Drop file1; the lock should be released.
                    drop(file1);
                    file2.lock_shared().unwrap();
                }

                #[$runtime]
                async fn test_open() {
                    let path = concat!($filename_prefix, "_open_test.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();

                    file.truncate(12).await.unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);
                    // mmap the file
                    let file = <$mmap_file>::open(path).await.unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[$runtime]
                async fn test_open_with_options() {
                    let path = concat!($filename_prefix, "_open_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();
                    file.truncate(23).await.unwrap();
                    file.write_all("sanity text".as_bytes(), 0).unwrap();
                    file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let opts = AsyncOptions::new()
                        // mmap content after the sanity text
                        .offset("sanity text".as_bytes().len() as u64);
                    // mmap the file
                    let file = <$mmap_file>::open_with_options(path, opts).await.unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[$runtime]
                async fn test_open_exec() {
                    let path = concat!($filename_prefix, "_open_exec.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();
                    file.truncate(12).await.unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);
                    // mmap the file
                    let file = <$mmap_file>::open_exec(path).await.unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[$runtime]
                async fn test_open_exec_with_options() {
                    let path = concat!($filename_prefix, "_open_exec_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();
                    file.truncate(23).await.unwrap();
                    file.write_all("sanity text".as_bytes(), 0).unwrap();
                    file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let opts = AsyncOptions::new()
                        // mmap content after the sanity text
                        .offset("sanity text".as_bytes().len() as u64);
                    // mmap the file
                    let file = <$mmap_file>::open_exec_with_options(path, opts).await.unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[$runtime]
                async fn test_remove() {
                    let path = concat!($filename_prefix, "_remove.txt");
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();

                    file.truncate(12).await.unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    file.remove().await.unwrap();

                    let err = File::open(path).await;
                    assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
                }

                #[$runtime]
                async fn test_close_with_truncate() {
                    let path = concat!($filename_prefix, "_close_with_truncate.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();

                    file.truncate(100).await.unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    file.close_with_truncate(50).await.unwrap();

                    let file = <$mmap_file_mut>::open(path).await.unwrap();
                    let meta = file.metadata().await.unwrap();
                    assert_eq!(meta.len(), 50);
                }

                #[$runtime]
                async fn test_create() {
                    let path = concat!($filename_prefix, "_create.txt");
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();
                    defer!(std::fs::remove_file(path).unwrap(););
                    file.truncate(12).await.unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                }

                #[$runtime]
                async fn test_create_with_options() {
                    let path = concat!($filename_prefix, "_create_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let opts = AsyncOptions::new()
                        // truncate to 100
                        .max_size(100);
                    let mut file = <$mmap_file_mut>::create_with_options(path, opts).await.unwrap();

                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                }

                #[$runtime]
                async fn test_open_mut() {
                    let path = concat!($filename_prefix, "_open_mut.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();

                    file.truncate(12).await.unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let mut file = <$mmap_file_mut>::open(path).await.unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.truncate("some modified data...".len() as u64).await.unwrap();
                    file.write_all("some modified data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // reopen to check content
                    let mut buf = vec![0; "some modified data...".len()];
                    let file = <$mmap_file_mut>::open(path).await.unwrap();
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                }

                #[$runtime]
                async fn test_open_mut_with_options() {
                    let path = concat!($filename_prefix, "_open_mut_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();
                    file.truncate(23).await.unwrap();
                    file.write_all("sanity text".as_bytes(), 0).unwrap();
                    file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let opts = AsyncOptions::new()
                        // allow read
                        .read(true)
                        // allow write
                        .write(true)
                        // allow append
                        .append(true)
                        // truncate to 100
                        .max_size(100)
                        // mmap content after the sanity text
                        .offset("sanity text".as_bytes().len() as u64);
                    let mut file = <$mmap_file_mut>::open_with_options(path, opts).await.unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.truncate(("some modified data...".len() + "sanity text".len()) as u64).await.unwrap();
                    file.write_all("some modified data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // reopen to check content
                    let mut buf = vec![0; "some modified data...".len()];
                    let file = <$mmap_file_mut>::open(path).await.unwrap();
                    // skip the sanity text
                    file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
                    assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                }

                #[$runtime]
                async fn test_open_exist() {
                    let path = concat!($filename_prefix, "_open_exist.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    // create a temp file
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();
                    file.truncate(12).await.unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let mut file = <$mmap_file_mut>::open_exist(path).await.unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.truncate("some modified data...".len() as u64).await.unwrap();
                    file.write_all("some modified data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);


                    // reopen to check content
                    let mut buf = vec![0; "some modified data...".len()];
                    let file = <$mmap_file_mut>::open(path).await.unwrap();
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                }

                #[$runtime]
                async fn test_open_exist_with_options() {
                    let path = concat!($filename_prefix, "_open_exist_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    // create a temp file
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();
                    file.truncate(23).await.unwrap();
                    file.write_all("sanity text".as_bytes(), 0).unwrap();
                    file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let opts = AsyncOptions::new()
                        // truncate to 100
                        .max_size(100)
                        // mmap content after the sanity text
                        .offset("sanity text".as_bytes().len() as u64);

                    let mut file = <$mmap_file_mut>::open_exist_with_options(path, opts).await.unwrap();

                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.truncate(("some modified data...".len() + "sanity text".len()) as u64).await.unwrap();
                    file.write_all("some modified data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();

                    // reopen to check content, cow will not change the content.
                    let file = <$mmap_file_mut>::open(path).await.unwrap();
                    let mut buf = vec![0; "some modified data...".len()];
                    // skip the sanity text
                    file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
                    assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
                }

                #[$runtime]
                async fn test_open_cow() {
                    let path = concat!($filename_prefix, "_open_cow.txt");
                    defer!(std::fs::remove_file(path).unwrap());

                    // create a temp file
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();

                    file.truncate(12).await.unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let mut file = <$mmap_file_mut>::open_cow(path).await.unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.write_all("some data!!!".as_bytes(), 0).unwrap();
                    file.flush().unwrap();

                    // cow, change will only be seen in current caller
                    assert_eq!(file.as_slice(), "some data!!!".as_bytes());
                    drop(file);

                    // reopen to check content, cow will not change the content.
                    let file = <$mmap_file_mut>::open(path).await.unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[$runtime]
                async fn test_open_cow_with_options() {
                    let path = concat!($filename_prefix, "_open_cow_with_options.txt");
                    defer!(std::fs::remove_file(path).unwrap());

                    // create a temp file
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();
                    file.truncate(23).await.unwrap();
                    file.write_all("sanity text".as_bytes(), 0).unwrap();
                    file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
                    file.flush().unwrap();
                    drop(file);

                    // mmap the file
                    let opts = AsyncOptions::new()
                        // mmap content after the sanity text
                        .offset("sanity text".as_bytes().len() as u64);

                    let mut file = <$mmap_file_mut>::open_cow_with_options(path, opts).await.unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    file.read_exact(buf.as_mut_slice(), 0).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());

                    // modify the file data
                    file.write_all("some data!!!".as_bytes(), 0).unwrap();
                    file.flush().unwrap();

                    // cow, change will only be seen in current caller
                    assert_eq!(file.as_slice(), "some data!!!".as_bytes());
                    drop(file);

                    // reopen to check content, cow will not change the content.
                    let file = <$mmap_file_mut>::open(path).await.unwrap();
                    let mut buf = vec![0; "some data...".len()];
                    // skip the sanity text
                    file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
                    assert_eq!(buf.as_slice(), "some data...".as_bytes());
                }

                #[$runtime]
                async fn test_freeze() {
                    let path = concat!($filename_prefix, "_freeze.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();
                    file.truncate(12).await.unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    // freeze
                    file.freeze().unwrap();
                }

                #[$runtime]
                async fn test_freeze_exec() {
                    let path = concat!($filename_prefix, "_freeze_exec.txt");
                    defer!(std::fs::remove_file(path).unwrap());
                    let mut file = <$mmap_file_mut>::create(path).await.unwrap();
                    file.truncate(12).await.unwrap();
                    file.write_all("some data...".as_bytes(), 0).unwrap();
                    file.flush().unwrap();
                    // freeze_exec
                    file.freeze_exec().unwrap();
                }
            }
        };
    }
}

mod disk;
mod empty;
/// Errors in this crate
pub mod error;
mod memory;
mod metadata;
pub use metadata::{MetaData, MetaDataExt};
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
