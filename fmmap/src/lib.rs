#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
#![allow(
  rustdoc::broken_intra_doc_links,
  unused_macros,
  clippy::len_without_is_empty,
  clippy::upper_case_acronyms
)]
#![deny(missing_docs)]
//
// # Safety of file-backed memory maps
//
// Every constructor in this crate that returns a *file-backed* memory map is
// `unsafe`. Memory-backed (`*::memory*`) and empty constructors are safe.
//
// The unsafety is intrinsic to the underlying mmap mechanism: while a process
// holds a memory map of a file, **another process or thread can mutate or
// truncate the same file at any time**. Concretely:
//
// * Another writer mapping the same file produces aliasing between
//   `&mut [u8]` slices yielded by `as_mut_slice()`. This violates Rust's
//   aliasing rules and is undefined behavior.
// * Another process truncating the file shrinks the valid backing pages of the
//   mapping. Reading or writing past the new EOF through the mapping signals
//   `SIGBUS` (Unix) or raises an SEH exception (Windows), which Rust treats as
//   undefined behavior because the abort happens outside Rust's control flow.
// * A read-only mapping can be invalidated the same way by an external writer
//   that extends or truncates the file.
//
// fmmap auto-acquires a `flock`-style file lock on every file-backed open
// (exclusive for writable mappings, shared for read-only) to *cooperate* with
// other fmmap handles in this and other processes. But `flock` is **advisory**:
// it does nothing against a `std::fs::File`, a `dd`, a `truncate(1)`, an editor
// rewriting in place, or any other non-cooperating accessor.
//
// As a result, the constructor's safety contract is:
//
// > The caller must ensure, by means outside this crate, that no other process
// > or thread will mutate or truncate the file for as long as this mapping
// > (and any borrowed slices it has yielded) is alive. Calling the constructor
// > otherwise is undefined behavior.
//
// In practice this usually means: own the file exclusively (e.g. it's a
// scratch file in a directory only your process writes to), or use OS-level
// mandatory locking, or accept the risk in a controlled environment.
//
// Memory-backed constructors (`*::memory*`) hold their bytes in `BytesMut`
// and are not subject to this contract — they are safe.
#[macro_use]
extern crate enum_dispatch;

macro_rules! cfg_smol {
  ($($item:item)*) => {
    $(
    #[cfg(feature = "smol")]
    #[cfg_attr(docsrs, doc(cfg(feature = "smol")))]
      $item
    )*
  }
}

macro_rules! cfg_tokio {
  ($($item:item)*) => {
    $(
    #[cfg(feature = "tokio")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
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
    #[cfg(any(feature = "smol", feature = "tokio"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "smol", feature = "tokio"))))]
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
    fn lock(&mut self) -> crate::error::Result<()> {
      Ok(())
    }

    #[inline]
    unsafe fn lock_shared(&mut self) -> crate::error::Result<()> {
      Ok(())
    }

    #[inline]
    fn try_lock(&mut self) -> crate::error::Result<()> {
      Ok(())
    }

    #[inline]
    unsafe fn try_lock_shared(&mut self) -> crate::error::Result<()> {
      Ok(())
    }

    #[inline]
    unsafe fn unlock(&mut self) -> crate::error::Result<()> {
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
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          // `defer!` runs at end of scope BEFORE locals declared after it
          // drop. Register cleanup first so `file1`'s mmap+File drop before
          // `remove_file` runs (Windows can't remove a still-mapped file).
          defer!(let _ = std::fs::remove_file(&path););
          let mut file1 = <$mmap_file_mut>::create_with_options(&path, Options::new().max_size(100)).unwrap();
          file1.write_all(vec![1; 100].as_slice(), 0).unwrap();
          file1.flush_range(0, 10).unwrap();
          file1.flush_async_range(11, 20).unwrap();
          file1.flush_async().unwrap();
          }
        }

        #[test]
        fn test_auto_lock() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let file1 = <$mmap_file_mut>::open(&path).unwrap();

          // A second writable mapping of the same file must fail because the
          // first holds an exclusive lock — without this the two mappings
          // would alias each other (UB).
          assert!(<$mmap_file_mut>::open(&path).is_err());
          // Same applies to read-only mappings while a writer holds the file.
          assert!(<$mmap_file>::open(&path).is_err());

          drop(file1);

          // Once the writer is dropped, multiple read-only mappings can
          // coexist (shared locks are compatible).
          let r1 = <$mmap_file>::open(&path).unwrap();
          let r2 = <$mmap_file>::open(&path).unwrap();
          // But a writer can't be opened while readers exist.
          assert!(<$mmap_file_mut>::open(&path).is_err());
          drop(r1);
          drop(r2);

          // After all readers drop, a writer can again be opened.
          let _ = <$mmap_file_mut>::open(&path).unwrap();
          }
        }

        #[test]
        fn test_open() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).unwrap();

          file.truncate(12).unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);
          // mmap the file
          let file = <$mmap_file>::open(&path).unwrap();
          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        #[test]
        fn test_open_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).unwrap();
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
          let file = <$mmap_file>::open_with_options(&path, opts).unwrap();
          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        // PROT_EXEC mmap of a regular file is forbidden on macOS without
        // a code-signed binary carrying com.apple.security.cs.allow-* JIT
        // entitlements. `cargo test` binaries don't qualify, so skip these
        // tests there. Linux and Windows accept it.
        #[test]
        #[cfg(not(target_os = "macos"))]
        fn test_open_exec() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).unwrap();
          file.truncate(12).unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);
          // mmap the file
          let file = <$mmap_file>::open_exec(&path).unwrap();
          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        #[test]
        #[cfg(not(target_os = "macos"))]
        fn test_open_exec_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).unwrap();
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
          let file = <$mmap_file>::open_exec_with_options(&path, opts).unwrap();
          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        #[test]
        fn test_remove() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          let mut file = <$mmap_file_mut>::create(&path).unwrap();

          file.truncate(12).unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          file.drop_remove().unwrap();

          let err = File::open(&path);
          assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
          }
        }

        #[test]
        fn test_close_with_truncate() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).unwrap();

          file.truncate(100).unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          file.close_with_truncate(50).unwrap();

          let file = <$mmap_file_mut>::open(&path).unwrap();
          let meta = file.metadata().unwrap();
          assert_eq!(meta.len(), 50);
          }
        }

        #[test]
        fn test_create() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          let mut file = <$mmap_file_mut>::create(&path).unwrap();
          defer!(let _ = std::fs::remove_file(&path););
          file.truncate(12).unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          }
        }

        #[test]
        fn test_create_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let opts = Options::new()
            // truncate to 100
            .max_size(100);
          let mut file = <$mmap_file_mut>::create_with_options(&path, opts).unwrap();

          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          }
        }

        #[test]
        fn test_open_mut() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).unwrap();

          file.truncate(12).unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);

          // mmap the file
          let mut file = <$mmap_file_mut>::open(&path).unwrap();
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
          let file = <$mmap_file_mut>::open(&path).unwrap();
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
          }
        }

        #[test]
        fn test_open_mut_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).unwrap();
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
          let mut file = <$mmap_file_mut>::open_with_options(&path, opts).unwrap();
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
          let file = <$mmap_file_mut>::open(&path).unwrap();
          // skip the sanity text
          file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
          assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
          }
        }

        #[test]
        fn test_open_exist() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          // create a temp file
          let mut file = <$mmap_file_mut>::create(&path).unwrap();
          file.truncate(12).unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);

          // mmap the file
          let mut file = <$mmap_file_mut>::open_exist(&path).unwrap();
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
          let file = <$mmap_file_mut>::open(&path).unwrap();
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
          }
        }

        #[test]
        fn test_open_exist_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          // create a temp file
          let mut file = <$mmap_file_mut>::create(&path).unwrap();
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

          let mut file = <$mmap_file_mut>::open_exist_with_options(&path, opts).unwrap();

          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());

          // modify the file data
          file.truncate(("some modified data...".len() + "sanity text".len()) as u64).unwrap();
          file.write_all("some modified data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);

          // reopen to check content, cow will not change the content.
          let file = <$mmap_file_mut>::open(&path).unwrap();
          let mut buf = vec![0; "some modified data...".len()];
          // skip the sanity text
          file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
          assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
          }
        }

        #[test]
        fn test_open_cow() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););

          // create a temp file
          let mut file = <$mmap_file_mut>::create(&path).unwrap();

          file.truncate(12).unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);

          // mmap the file
          let mut file = <$mmap_file_mut>::open_cow(&path).unwrap();
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
          let file = <$mmap_file_mut>::open(&path).unwrap();
          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        #[test]
        fn test_open_cow_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););

          // create a temp file
          let mut file = <$mmap_file_mut>::create(&path).unwrap();
          file.truncate(23).unwrap();
          file.write_all("sanity text".as_bytes(), 0).unwrap();
          file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
          file.flush().unwrap();
          drop(file);

          // mmap the file
          let opts = Options::new()
            // mmap content after the sanity text
            .offset("sanity text".as_bytes().len() as u64);

          let mut file = <$mmap_file_mut>::open_cow_with_options(&path, opts).unwrap();
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
          let file = <$mmap_file_mut>::open(&path).unwrap();
          let mut buf = vec![0; "some data...".len()];
          // skip the sanity text
          file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        #[test]
        fn test_freeze() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).unwrap();
          file.truncate(12).unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          // freeze
          file.freeze().unwrap();
          }
        }

        #[test]
        fn test_freeze_exec() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).unwrap();
          file.truncate(12).unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          // freeze_exec
          file.freeze_exec().unwrap();
          }
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
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file1 = <$mmap_file_mut>::create_with_options(&path, AsyncOptions::new().max_size(100)).await.unwrap();
          file1.write_all(vec![1; 100].as_slice(), 0).unwrap();
          file1.flush_range(0, 10).unwrap();
          file1.flush_async_range(11, 20).unwrap();
          file1.flush_async().unwrap();
          }
        }

        #[$runtime]
        async fn test_auto_lock() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let file1 = <$mmap_file_mut>::open(&path).await.unwrap();

          // A second writable mapping of the same file must fail because the
          // first holds an exclusive lock — without this the two mappings
          // would alias each other (UB).
          assert!(<$mmap_file_mut>::open(&path).await.is_err());
          // Same applies to read-only mappings while a writer holds the file.
          assert!(<$mmap_file>::open(&path).await.is_err());

          drop(file1);

          // Once the writer is dropped, multiple read-only mappings can
          // coexist (shared locks are compatible).
          let r1 = <$mmap_file>::open(&path).await.unwrap();
          let r2 = <$mmap_file>::open(&path).await.unwrap();
          // But a writer can't be opened while readers exist.
          assert!(<$mmap_file_mut>::open(&path).await.is_err());
          drop(r1);
          drop(r2);

          // After all readers drop, a writer can again be opened.
          let _ = <$mmap_file_mut>::open(&path).await.unwrap();
          }
        }

        #[$runtime]
        async fn test_open() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();

          file.truncate(12).await.unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);
          // mmap the file
          let file = <$mmap_file>::open(&path).await.unwrap();
          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        #[$runtime]
        async fn test_open_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();
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
          let file = <$mmap_file>::open_with_options(&path, opts).await.unwrap();
          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        // PROT_EXEC mmap of a regular file is forbidden on macOS without
        // a code-signed binary carrying com.apple.security.cs.allow-* JIT
        // entitlements. `cargo test` binaries don't qualify, so skip these
        // tests there. Linux and Windows accept it.
        #[$runtime]
        #[cfg(not(target_os = "macos"))]
        async fn test_open_exec() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();
          file.truncate(12).await.unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);
          // mmap the file
          let file = <$mmap_file>::open_exec(&path).await.unwrap();
          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        #[$runtime]
        #[cfg(not(target_os = "macos"))]
        async fn test_open_exec_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();
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
          let file = <$mmap_file>::open_exec_with_options(&path, opts).await.unwrap();
          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        #[$runtime]
        async fn test_remove() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();

          file.truncate(12).await.unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          file.drop_remove().await.unwrap();

          let err = File::open(&path).await;
          assert_eq!(err.unwrap_err().kind(), std::io::ErrorKind::NotFound);
          }
        }

        #[$runtime]
        async fn test_close_with_truncate() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();

          file.truncate(100).await.unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          file.close_with_truncate(50).await.unwrap();

          let file = <$mmap_file_mut>::open(&path).await.unwrap();
          let meta = file.metadata().await.unwrap();
          assert_eq!(meta.len(), 50);
          }
        }

        #[$runtime]
        async fn test_create() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();
          defer!(let _ = std::fs::remove_file(&path););
          file.truncate(12).await.unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          }
        }

        #[$runtime]
        async fn test_create_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let opts = AsyncOptions::new()
            // truncate to 100
            .max_size(100);
          let mut file = <$mmap_file_mut>::create_with_options(&path, opts).await.unwrap();

          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          }
        }

        #[$runtime]
        async fn test_open_mut() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();

          file.truncate(12).await.unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);

          // mmap the file
          let mut file = <$mmap_file_mut>::open(&path).await.unwrap();
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
          let file = <$mmap_file_mut>::open(&path).await.unwrap();
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
          }
        }

        #[$runtime]
        async fn test_open_mut_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();
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
          let mut file = <$mmap_file_mut>::open_with_options(&path, opts).await.unwrap();
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
          let file = <$mmap_file_mut>::open(&path).await.unwrap();
          // skip the sanity text
          file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
          assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
          }
        }

        #[$runtime]
        async fn test_open_exist() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          // create a temp file
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();
          file.truncate(12).await.unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);

          // mmap the file
          let mut file = <$mmap_file_mut>::open_exist(&path).await.unwrap();
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
          let file = <$mmap_file_mut>::open(&path).await.unwrap();
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
          }
        }

        #[$runtime]
        async fn test_open_exist_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          // create a temp file
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();
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

          let mut file = <$mmap_file_mut>::open_exist_with_options(&path, opts).await.unwrap();

          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());

          // modify the file data
          file.truncate(("some modified data...".len() + "sanity text".len()) as u64).await.unwrap();
          file.write_all("some modified data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);

          // reopen to check content, cow will not change the content.
          let file = <$mmap_file_mut>::open(&path).await.unwrap();
          let mut buf = vec![0; "some modified data...".len()];
          // skip the sanity text
          file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
          assert_eq!(buf.as_slice(), "some modified data...".as_bytes());
          }
        }

        #[$runtime]
        async fn test_open_cow() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););

          // create a temp file
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();

          file.truncate(12).await.unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          drop(file);

          // mmap the file
          let mut file = <$mmap_file_mut>::open_cow(&path).await.unwrap();
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
          let file = <$mmap_file_mut>::open(&path).await.unwrap();
          let mut buf = vec![0; "some data...".len()];
          file.read_exact(buf.as_mut_slice(), 0).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        #[$runtime]
        async fn test_open_cow_with_options() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););

          // create a temp file
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();
          file.truncate(23).await.unwrap();
          file.write_all("sanity text".as_bytes(), 0).unwrap();
          file.write_all("some data...".as_bytes(), "sanity text".as_bytes().len()).unwrap();
          file.flush().unwrap();
          drop(file);

          // mmap the file
          let opts = AsyncOptions::new()
            // mmap content after the sanity text
            .offset("sanity text".as_bytes().len() as u64);

          let mut file = <$mmap_file_mut>::open_cow_with_options(&path, opts).await.unwrap();
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
          let file = <$mmap_file_mut>::open(&path).await.unwrap();
          let mut buf = vec![0; "some data...".len()];
          // skip the sanity text
          file.read_exact(buf.as_mut_slice(), "sanity text".as_bytes().len()).unwrap();
          assert_eq!(buf.as_slice(), "some data...".as_bytes());
          }
        }

        #[$runtime]
        async fn test_freeze() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();
          file.truncate(12).await.unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          // freeze
          file.freeze().unwrap();
          }
        }

        #[$runtime]
        async fn test_freeze_exec() {
          #[allow(unused_unsafe)]
          unsafe {
          let path = crate::tests::get_random_filename();
          defer!(let _ = std::fs::remove_file(&path););
          let mut file = <$mmap_file_mut>::create(&path).await.unwrap();
          file.truncate(12).await.unwrap();
          file.write_all("some data...".as_bytes(), 0).unwrap();
          file.flush().unwrap();
          // freeze_exec
          file.freeze_exec().unwrap();
          }
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
mod tests;
/// File I/O utils function
pub mod utils;
mod writer;

cfg_sync!(
  /// std based mmap file
  pub mod sync {
    pub use crate::{
      mmap_file::{MmapFile, MmapFileExt, MmapFileMut, MmapFileMutExt},
      options::Options,
      reader::{MmapFileReader, MmapFileReaderExt},
      writer::{MmapFileWriter, MmapFileWriterExt},
    };
  }

  pub use reader::{MmapFileReader, MmapFileReaderExt};
  pub use writer::{MmapFileWriter, MmapFileWriterExt};
  pub use mmap_file::{MmapFileExt, MmapFileMutExt, MmapFile, MmapFileMut};
  pub use options::Options;
);

cfg_smol!(
  /// smol based mmap file
  pub mod smol {
    pub use crate::{
      mmap_file::smol_impl::{
        AsyncMmapFile, AsyncMmapFileExt, AsyncMmapFileMut, AsyncMmapFileMutExt,
      },
      options::smol_impl::AsyncOptions,
      reader::smol_impl::AsyncMmapFileReader,
      writer::smol_impl::AsyncMmapFileWriter,
    };
  }
);

cfg_tokio!(
  /// tokio based mmap file
  pub mod tokio {
    pub use crate::{
      mmap_file::tokio_impl::{
        AsyncMmapFile, AsyncMmapFileExt, AsyncMmapFileMut, AsyncMmapFileMutExt,
      },
      options::tokio_impl::AsyncOptions,
      reader::tokio_impl::AsyncMmapFileReader,
      writer::tokio_impl::AsyncMmapFileWriter,
    };
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
      pub use crate::{
        disk::{DiskMmapFile, DiskMmapFileMut},
        memory::{MemoryMmapFile, MemoryMmapFileMut},
      };
    }
    pub use crate::disk::{DiskMmapFile, DiskMmapFileMut};
    pub use crate::memory::{MemoryMmapFile, MemoryMmapFileMut};
  );

  cfg_smol!(
    /// smol based raw mmap file
    ///
    /// Inner components of [`AsyncMmapFile`], [`AsyncMmapFileMut`]
    ///
    /// [`AsyncMmapFile`]: smol/struct.AsyncMmapFile.html
    /// [`AsyncMmapFileMut`]: smol/struct.AsyncMmapFileMut.html
    pub mod smol {
      pub use crate::{
        disk::smol_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut},
        memory::smol_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut},
      };
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
      pub use crate::{
        disk::tokio_impl::{AsyncDiskMmapFile, AsyncDiskMmapFileMut},
        memory::tokio_impl::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut},
      };
    }
  );
}
