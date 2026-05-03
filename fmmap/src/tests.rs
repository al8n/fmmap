use std::{
  path::PathBuf,
  sync::atomic::{AtomicU64, Ordering},
  time::{SystemTime, UNIX_EPOCH},
};

/// Returns a unique path for a test temp file. The path is in the OS temp
/// dir, uses a flat filename (no subdir) to avoid `create_dir` permission
/// surprises on Windows runners, and combines pid + nanosecond timestamp +
/// a process-local counter so it's unique even across `cargo hack
/// --each-feature` invocations and parallel test threads.
#[allow(dead_code)]
pub fn get_random_filename() -> PathBuf {
  static COUNTER: AtomicU64 = AtomicU64::new(0);
  let mut filename = std::env::temp_dir();
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_nanos())
    .unwrap_or(0);
  filename.push(format!(
    "fmmap-{}-{}-{}",
    std::process::id(),
    nanos,
    COUNTER.fetch_add(1, Ordering::Relaxed),
  ));
  filename.set_extension("txt");
  filename
}

#[cfg(feature = "sync")]
mod sync {
  macro_rules! sync_tests {
    ($([$test_fn: ident, $init: block]), +$(,)?) => {
      use std::io::{Read, Seek, SeekFrom, Write};
      use scopeguard::defer;

      const SANITY_TEXT: &'static str = "Hello, sync file!";
      const MODIFIED_SANITY_TEXT: &'static str = "Hello, modified sync file!";

      $(
      #[test]
      fn $test_fn() {
        let mut file = $init;
        assert_eq!(file.as_mut_slice().len(), 0);
        file.truncate(8096).unwrap(); // 1 KB
        let mut writter = file.writer(0).unwrap();
        writter.write_all(SANITY_TEXT.as_bytes()).unwrap();
        writter.seek(SeekFrom::Start(100)).unwrap();
        writter.write_i8(-8).unwrap();
        writter.write_i16(-16).unwrap();
        writter.write_i16_le(-61).unwrap();
        writter.write_i32(-32).unwrap();
        writter.write_i32_le(-23).unwrap();
        writter.write_i64(-64).unwrap();
        writter.write_i64_le(-46).unwrap();
        writter.write_isize(-64).unwrap();
        writter.write_isize_le(-46).unwrap();
        writter.write_i128(-128).unwrap();
        writter.write_i128_le(-821).unwrap();
        writter.write_f32(32.0).unwrap();
        writter.write_f32_le(23.0).unwrap();
        writter.write_f64(64.0).unwrap();
        writter.write_f64_le(46.0).unwrap();
        writter.flush().unwrap();
        writter.seek(SeekFrom::End(0)).unwrap();
        let mut reader = file.reader(0).unwrap();
        let mut buf = [0; SANITY_TEXT.len()];
        reader.read_exact(&mut buf).unwrap();
        assert!(buf.eq(SANITY_TEXT.as_bytes()));
        reader.seek(SeekFrom::Start(100)).unwrap();
        assert_eq!(-8, reader.read_i8().unwrap());
        assert_eq!(-16, reader.read_i16().unwrap());
        assert_eq!(-61, reader.read_i16_le().unwrap());
        assert_eq!(-32, reader.read_i32().unwrap());
        assert_eq!(-23, reader.read_i32_le().unwrap());
        assert_eq!(-64, reader.read_i64().unwrap());
        assert_eq!(-46, reader.read_i64_le().unwrap());
        assert_eq!(-64, reader.read_isize().unwrap());
        assert_eq!(-46, reader.read_isize_le().unwrap());
        assert_eq!(-128, reader.read_i128().unwrap());
        assert_eq!(-821, reader.read_i128_le().unwrap());
        assert_eq!(32.0, reader.read_f32().unwrap());
        assert_eq!(23.0, reader.read_f32_le().unwrap());
        assert_eq!(64.0, reader.read_f64().unwrap());
        assert_eq!(46.0, reader.read_f64_le().unwrap());

        let mut range_writer = file.range_writer(7800, 96).unwrap();
        range_writer.write_u8(8).unwrap();
        range_writer.write_u16(16).unwrap();
        range_writer.write_u16_le(61).unwrap();
        range_writer.write_u32(32).unwrap();
        range_writer.write_u32_le(23).unwrap();
        range_writer.write_u64(64).unwrap();
        range_writer.write_u64_le(46).unwrap();
        range_writer.write_usize(64).unwrap();
        range_writer.write_usize_le(46).unwrap();
        range_writer.write_u128(128).unwrap();
        range_writer.write_u128_le(821).unwrap();
        range_writer.flush().unwrap();

        let mut range_reader = file.range_reader(7800, 96).unwrap();
        assert_eq!(8, range_reader.read_u8().unwrap());
        assert_eq!(16, range_reader.read_u16().unwrap());
        assert_eq!(61, range_reader.read_u16_le().unwrap());
        assert_eq!(32, range_reader.read_u32().unwrap());
        assert_eq!(23, range_reader.read_u32_le().unwrap());
        assert_eq!(64, range_reader.read_u64().unwrap());
        assert_eq!(46, range_reader.read_u64_le().unwrap());
        assert_eq!(64, range_reader.read_usize().unwrap());
        assert_eq!(46, range_reader.read_usize_le().unwrap());
        assert_eq!(128, range_reader.read_u128().unwrap());
        assert_eq!(821, range_reader.read_u128_le().unwrap());

        file.write_u8(8, 1000).unwrap();
        file.write_u16(16, 1001).unwrap();
        file.write_u32(32, 1003).unwrap();
        file.write_u64(64, 1007).unwrap();
        file.write_u128(128, 1015).unwrap();
        file.write_u16_le(16, 1031).unwrap();
        file.write_u32_le(32, 1033).unwrap();
        file.write_u64_le(64, 1037).unwrap();
        file.write_u128_le(128, 1045).unwrap();
        file.write_usize(64, 1061).unwrap();
        file.write_usize_le(64, 1069).unwrap();

        assert_eq!(8, file.read_u8(1000).unwrap());
        assert_eq!(16, file.read_u16(1001).unwrap());
        assert_eq!(32, file.read_u32(1003).unwrap());
        assert_eq!(64, file.read_u64(1007).unwrap());
        assert_eq!(128, file.read_u128(1015).unwrap());
        assert_eq!(16, file.read_u16_le(1031).unwrap());
        assert_eq!(32, file.read_u32_le(1033).unwrap());
        assert_eq!(64, file.read_u64_le(1037).unwrap());
        assert_eq!(128, file.read_u128_le(1045).unwrap());
        assert_eq!(64, file.read_usize(1061).unwrap());
        assert_eq!(64, file.read_usize_le(1069).unwrap());

        file.write_i8(-8, 2000).unwrap();
        file.write_i16(-16, 2001).unwrap();
        file.write_i32(-32, 2003).unwrap();
        file.write_i64(-64, 2007).unwrap();
        file.write_i128(-128, 2015).unwrap();
        file.write_i16_le(-16, 2031).unwrap();
        file.write_i32_le(-32, 2033).unwrap();
        file.write_i64_le(-64, 2037).unwrap();
        file.write_i128_le(-128, 2045).unwrap();
        file.write_isize(-64, 2061).unwrap();
        file.write_isize_le(-64, 2069).unwrap();

        assert_eq!(-8, file.read_i8(2000).unwrap());
        assert_eq!(-16, file.read_i16(2001).unwrap());
        assert_eq!(-32, file.read_i32(2003).unwrap());
        assert_eq!(-64, file.read_i64(2007).unwrap());
        assert_eq!(-128, file.read_i128(2015).unwrap());
        assert_eq!(-16, file.read_i16_le(2031).unwrap());
        assert_eq!(-32, file.read_i32_le(2033).unwrap());
        assert_eq!(-64, file.read_i64_le(2037).unwrap());
        assert_eq!(-128, file.read_i128_le(2045).unwrap());
        assert_eq!(-64, file.read_isize(2061).unwrap());
        assert_eq!(-64, file.read_isize_le(2069).unwrap());

        file.write_f32(32.0, 3000).unwrap();
        file.write_f32_le(32.0, 3004).unwrap();
        file.write_f64(64.0, 3008).unwrap();
        file.write_f64_le(64.0, 3016).unwrap();
        assert_eq!(32.0, file.read_f32(3000).unwrap());
        assert_eq!(32.0, file.read_f32_le(3004).unwrap());
        assert_eq!(64.0, file.read_f64(3008).unwrap());
        assert_eq!(64.0, file.read_f64_le(3016).unwrap());

        file.zero_range(3000, 3024);

        file.truncate(0).unwrap();
        file.truncate(100).unwrap();

        let st = file.bytes_mut(0, SANITY_TEXT.len()).unwrap();
        st.copy_from_slice(SANITY_TEXT.as_bytes());

        let n = file.write(MODIFIED_SANITY_TEXT.as_bytes(), 0);
        assert_eq!(n, MODIFIED_SANITY_TEXT.len());

        let mst = file.bytes(0, MODIFIED_SANITY_TEXT.len()).unwrap();
        assert_eq!(mst, MODIFIED_SANITY_TEXT.as_bytes());

        let mut vec = vec![0; MODIFIED_SANITY_TEXT.len()];
        let n = file.read(vec.as_mut_slice(), 0);
        assert_eq!(n, MODIFIED_SANITY_TEXT.len());

        let sm = file.slice_mut(MODIFIED_SANITY_TEXT.len(), 4);
        sm.copy_from_slice(&32u32.to_be_bytes());

        let buf = file.slice(MODIFIED_SANITY_TEXT.len(), 4);
        let n = u32::from_be_bytes(buf.try_into().unwrap());
        assert_eq!(n, 32);

        let v = file.copy_all_to_vec();
        assert_eq!(v.len(), 100);
        assert_eq!(&v[..MODIFIED_SANITY_TEXT.len()], MODIFIED_SANITY_TEXT.as_bytes());
        let v = file.copy_range_to_vec(0, MODIFIED_SANITY_TEXT.len());
        assert_eq!(v.as_slice(), MODIFIED_SANITY_TEXT.as_bytes());

        let pb = get_random_filename();
        file.write_all_to_new_file(&pb).unwrap();
        defer!(let _ = std::fs::remove_file(&pb););

        let pb1 = get_random_filename();
        defer!(let _ = std::fs::remove_file(&pb1););
        file.write_range_to_new_file(&pb1, 0, MODIFIED_SANITY_TEXT.len()).unwrap();

        let mut file = std::fs::File::open(&pb).unwrap();
        assert_eq!(file.metadata().unwrap().len(), 100);
        let mut buf = vec![0; MODIFIED_SANITY_TEXT.len()];
        file.read_exact(&mut buf).unwrap();
        assert_eq!(buf.as_slice(), MODIFIED_SANITY_TEXT.as_bytes());
        drop(file);

        let mut file = std::fs::File::open(&pb1).unwrap();
        assert_eq!(file.metadata().unwrap().len(), MODIFIED_SANITY_TEXT.len() as u64);
        let mut buf = vec![0; MODIFIED_SANITY_TEXT.len()];
        file.read_exact(&mut buf).unwrap();
        assert_eq!(buf.as_slice(), MODIFIED_SANITY_TEXT.as_bytes());
        drop(file);
      }
      )*
    };
    }

  use super::*;
  use crate::{
    raw::{DiskMmapFileMut, MemoryMmapFileMut},
    MmapFileExt, MmapFileMut, MmapFileMutExt, MmapFileReaderExt, MmapFileWriterExt,
  };

  sync_tests!(
    [test_memory_file_mut, {
      MemoryMmapFileMut::new("memory.txt")
    }],
    [test_mmap_file_mut, {
      let mut file =
        MmapFileMut::from(unsafe { DiskMmapFileMut::create(get_random_filename()) }.unwrap());
      file.set_remove_on_drop(true);
      assert!(file.get_remove_on_drop());
      file
    }],
  );

  #[test]
  fn read_returns_partial_count_at_eof() {
    let file = MemoryMmapFileMut::from_vec("memory.txt", vec![1, 2, 3, 4]);
    let mut dst = [0; 8];

    let n = file.read(&mut dst, 2);

    assert_eq!(n, 2);
    assert_eq!(&dst[..2], &[3, 4]);
    assert_eq!(&dst[2..], &[0; 6]);
  }

  #[test]
  fn checked_range_methods_reject_overflow() {
    let mut file = MemoryMmapFileMut::from_vec("memory.txt", vec![1, 2, 3, 4]);
    let path = get_random_filename();

    assert!(file.bytes(usize::MAX, 1).is_err());
    assert!(file.range_reader(usize::MAX, 1).is_err());
    assert!(file.write_range_to_new_file(&path, usize::MAX, 1).is_err());
    assert!(file.bytes_mut(usize::MAX, 1).is_err());
    assert!(file.range_writer(usize::MAX, 1).is_err());
  }

  #[test]
  fn bytes_mut_allows_exact_fit() {
    let mut file = MemoryMmapFileMut::from_vec("memory.txt", vec![1, 2, 3, 4]);

    assert_eq!(file.bytes_mut(0, 4).unwrap().len(), 4);
    assert_eq!(file.bytes_mut(4, 0).unwrap().len(), 0);
    assert_eq!(file.writer(4).unwrap().len(), 0);
  }

  // -- coverage: memory file constructors ---------------------------------

  use crate::{raw::MemoryMmapFile, MetaDataExt, MmapFile};
  use bytes::Bytes;
  use std::path::Path;

  #[test]
  fn memory_mmap_file_constructors() {
    let f = MemoryMmapFile::new("a.mem", Bytes::from_static(b"abc"));
    assert_eq!(f.path(), Path::new("a.mem"));
    assert_eq!(f.as_slice(), b"abc");

    let f = MemoryMmapFile::from_vec("b.mem", vec![1, 2, 3]);
    assert_eq!(f.as_slice(), &[1, 2, 3]);

    let f = MemoryMmapFile::from_string("c.mem", "hello".to_string());
    assert_eq!(f.as_slice(), b"hello");

    let f = MemoryMmapFile::from_slice("d.mem", b"world");
    assert_eq!(f.as_slice(), b"world");

    let f = MemoryMmapFile::from_str("e.mem", "static");
    assert_eq!(f.as_slice(), b"static");

    let f = MemoryMmapFile::copy_from_slice("f.mem", b"copy");
    assert_eq!(f.as_slice(), b"copy");

    let bytes = f.into_bytes();
    assert_eq!(bytes.as_ref(), b"copy");
  }

  #[test]
  fn memory_mmap_file_mut_constructors() {
    let f = MemoryMmapFileMut::new("new.mem");
    assert_eq!(f.path(), Path::new("new.mem"));
    assert_eq!(f.as_slice().len(), 0);

    let f = MemoryMmapFileMut::with_capacity("cap.mem", 100);
    assert_eq!(f.path(), Path::new("cap.mem"));

    let f = MemoryMmapFileMut::from_vec("v.mem", vec![1, 2, 3]);
    assert_eq!(f.as_slice(), &[1, 2, 3]);

    let f = MemoryMmapFileMut::from_string("s.mem", "data".to_string());
    assert_eq!(f.as_slice(), b"data");

    let f = MemoryMmapFileMut::from_str("st.mem", "static");
    assert_eq!(f.as_slice(), b"static");

    let f = MemoryMmapFileMut::from_slice("sl.mem", b"slice");
    assert_eq!(f.as_slice(), b"slice");

    // freeze converts to immutable
    let frozen = f.freeze();
    assert_eq!(frozen.as_slice(), b"slice");

    // into_bytes_mut takes ownership
    let f = MemoryMmapFileMut::from_vec("foo.mem", vec![9, 8, 7]);
    let bm = f.into_bytes_mut();
    assert_eq!(bm.as_ref(), &[9, 8, 7]);
  }

  #[test]
  fn mmap_file_memory_constructors() {
    let f = MmapFile::memory("a", Bytes::from_static(b"abc"));
    assert_eq!(f.as_slice(), b"abc");

    let f = MmapFile::memory_from_vec("b", vec![1, 2]);
    assert_eq!(f.as_slice(), &[1, 2]);

    let f = MmapFile::memory_from_string("c", "hi".to_string());
    assert_eq!(f.as_slice(), b"hi");

    let f = MmapFile::memory_from_slice("d", b"static");
    assert_eq!(f.as_slice(), b"static");

    let f = MmapFile::memory_from_str("e", "str");
    assert_eq!(f.as_slice(), b"str");

    let f = MmapFile::memory_copy_from_slice("g", b"copy");
    assert_eq!(f.as_slice(), b"copy");
  }

  #[test]
  fn mmap_file_mut_memory_constructors() {
    let f = MmapFileMut::memory("a");
    assert_eq!(f.as_slice().len(), 0);

    let f = MmapFileMut::memory_with_capacity("b", 100);
    assert_eq!(f.path(), Path::new("b"));

    let f = MmapFileMut::memory_from_vec("c", vec![1, 2]);
    assert_eq!(f.as_slice(), &[1, 2]);

    let f = MmapFileMut::memory_from_string("d", "hi".to_string());
    assert_eq!(f.as_slice(), b"hi");

    let f = MmapFileMut::memory_from_slice("e", b"static");
    assert_eq!(f.as_slice(), b"static");

    let f = MmapFileMut::memory_from_str("f", "str");
    assert_eq!(f.as_slice(), b"str");

    // Exercise the Memory-arm of the wrapper's MmapFileMutExt dispatchers
    // (as_mut_slice, is_cow, flush_*, len).
    let mut f = MmapFileMut::memory_from_vec("disp.mem", vec![1, 2, 3]);
    let _ = f.as_mut_slice();
    assert!(!f.is_cow());
    f.flush().unwrap();
    f.flush_async().unwrap();
    f.flush_range(0, 1).unwrap();
    f.flush_async_range(0, 1).unwrap();
    assert_eq!(MmapFileExt::len(&f), 3);
  }

  #[test]
  fn memory_variant_lock_methods_are_noops() {
    let mut f = MmapFileMut::memory_from_vec("lock.mem", vec![1, 2, 3]);
    f.lock().unwrap();
    unsafe { f.lock_shared().unwrap() };
    f.try_lock().unwrap();
    unsafe { f.try_lock_shared().unwrap() };
    unsafe { f.unlock().unwrap() };

    let mut f = MmapFile::memory_from_vec("rlock.mem", vec![4, 5]);
    f.lock().unwrap();
    unsafe { f.lock_shared().unwrap() };
    f.try_lock().unwrap();
    unsafe { f.try_lock_shared().unwrap() };
    unsafe { f.unlock().unwrap() };
  }

  #[test]
  fn memory_file_metadata_and_accessors() {
    let f = MemoryMmapFile::from_str("meta.mem", "data");
    let meta = f.metadata().unwrap();
    assert_eq!(meta.len(), 4);
    // Trait `len` directly on MemoryMmapFile — covers `impl MmapFileExt`
    // body (vs going through `as_slice().len()` which hits `as_slice`).
    assert_eq!(MmapFileExt::len(&f), 4);

    let f = MemoryMmapFileMut::from_str("metam.mem", "data");
    let meta = f.metadata().unwrap();
    assert_eq!(meta.len(), 4);
    assert!(!f.is_exec());
    assert_eq!(MmapFileExt::len(&f), 4);

    // into_bytes on the Mut variant returns frozen Bytes
    let f = MemoryMmapFileMut::from_str("ib.mem", "abc");
    let b = f.into_bytes();
    assert_eq!(b.as_ref(), b"abc");
  }

  #[test]
  fn options_builder_branches() {
    use crate::Options;

    // exercise every builder method (most are mirrored to file_opts)
    let _ = Options::new()
      .max_size(1024)
      .read(true)
      .write(true)
      .create(false)
      .create_new(false)
      .append(false)
      .truncate(false)
      .offset(0)
      .len(64)
      .populate()
      .stack();

    // Default::default → Self::new
    let _ = Options::default();

    #[cfg(unix)]
    {
      let _ = Options::new().mode(0o644).custom_flags(0);
    }
  }

  #[test]
  fn memory_variant_freeze_and_close_and_remove() {
    // freeze: hits the MmapFileMutInner::Memory match arm
    let f = MmapFileMut::memory_from_vec("freeze.mem", vec![1, 2, 3]);
    let frozen = f.freeze().unwrap();
    assert_eq!(frozen.as_slice(), &[1, 2, 3]);

    // close on a memory variant: hits the `_ => Ok(())` arm
    let f = MmapFileMut::memory_from_vec("close.mem", vec![4, 5]);
    f.close_with_truncate(-1).unwrap();

    // remove on a memory variant: hits the `_ => { self.deleted = true; Ok(()) }`
    let f = MmapFileMut::memory_from_vec("rm.mem", vec![6, 7]);
    f.drop_remove().unwrap();
  }

  #[test]
  fn disk_wrapper_lock_method_dispatch() {
    use scopeguard::defer;
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    // Mut variant — exercises Disk arm of every lock dispatcher. The
    // public lock methods are now reentrant-safe (no-op when state
    // matches, WouldBlock when it mismatches), so no unlock-dance is
    // needed.
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    let _ = f.lock();
    let _ = unsafe { f.lock_shared() };
    let _ = f.try_lock();
    let _ = unsafe { f.try_lock_shared() };
    let _ = unsafe { f.unlock() };
    drop(f);
  }

  #[test]
  fn disk_read_only_wrapper_methods() {
    use scopeguard::defer;
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    // Pre-populate
    {
      let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
      f.truncate(8).unwrap();
      f.write_all(b"hello!!!", 0).unwrap();
      f.flush().unwrap();
    }

    // Read-only wrapper — exercises Disk arm of MmapFileInner dispatchers.
    // Reentrant-safe lock methods, so no unlock-dance needed.
    let mut f = unsafe { MmapFile::open(&path) }.unwrap();
    assert_eq!(f.as_slice(), b"hello!!!");
    assert_eq!(MmapFileExt::len(&f), 8);
    assert_eq!(f.path(), path.as_path());
    assert!(!f.is_exec());
    let _ = f.metadata().unwrap();
    let _ = f.lock();
    let _ = unsafe { f.lock_shared() };
    let _ = f.try_lock();
    let _ = unsafe { f.try_lock_shared() };
    let _ = unsafe { f.unlock() };
    drop(f);

    // Raw immutable disk type directly — covers `impl_mmap_file_ext!`
    // accessor arms not reached via wrapper enum dispatch.
    use crate::raw::DiskMmapFile;
    let mut raw = unsafe { DiskMmapFile::open(&path) }.unwrap();
    assert_eq!(MmapFileExt::len(&raw), 8);
    assert_eq!(MmapFileExt::as_slice(&raw), b"hello!!!");
    assert_eq!(MmapFileExt::path(&raw), path.as_path());
    assert!(!MmapFileExt::is_exec(&raw));
    let _ = MmapFileExt::metadata(&raw).unwrap();
    let _ = MmapFileExt::lock(&mut raw);
    let _ = unsafe { MmapFileExt::lock_shared(&mut raw) };
    let _ = MmapFileExt::try_lock(&mut raw);
    let _ = unsafe { MmapFileExt::try_lock_shared(&mut raw) };
    let _ = unsafe { MmapFileExt::unlock(&mut raw) };
  }

  #[test]
  fn utils_helpers_smoke() {
    use crate::utils::*;
    use scopeguard::defer;

    // Pre-create a target file so the open_* helpers find something.
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    {
      let _ = create_file(&path).unwrap();
    }

    let _ = open_read_only_file(&path).unwrap();
    let _ = open_exist_file(&path).unwrap();
    let _ = open_exist_file_with_append(&path).unwrap();
    let _ = open_or_create_file(&path).unwrap();

    // Truncate-mode open + sync_dir / sync_parent
    let _ = open_file_with_truncate(&path).unwrap();
    sync_dir(path.parent().unwrap()).unwrap();
    sync_parent(&path).unwrap();

    // not-a-directory error path: pass a regular file to sync_dir.
    assert!(sync_dir(&path).is_err());
  }

  #[test]
  fn disk_wrapper_close_with_truncate_negative_max_sz() {
    use scopeguard::defer;
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();
    f.write_all(b"abcdefgh", 0).unwrap();
    f.flush().unwrap();
    // max_sz < 0: skip the set_len/sync_parent branch
    f.close_with_truncate(-1).unwrap();
  }

  #[test]
  fn inherent_close_and_remove_on_disk_and_memory() {
    use scopeguard::defer;
    // disk + close(>=0): exercises the disk arm of `MmapFileMut::close`
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();
    f.write_all(b"abcdefgh", 0).unwrap();
    f.flush().unwrap();
    f.close(4).unwrap();
    // After close, inner is Empty. Methods on f still dispatch — exercises
    // the Empty arms of every dispatcher.
    let _ = f.len();
    let _ = f.as_slice();
    let _ = f.path();
    let _ = f.is_exec();
    let _ = f.metadata();
    let _ = f.lock();
    let _ = unsafe { f.lock_shared() };
    let _ = f.try_lock();
    let _ = unsafe { f.try_lock_shared() };
    let _ = unsafe { f.unlock() };

    // disk + close(<0): skips set_len branch
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.truncate(8).unwrap();
    f.write_all(b"abcdefgh", 0).unwrap();
    f.flush().unwrap();
    f.close(-1).unwrap();

    // disk + remove
    let path = get_random_filename();
    let mut f = unsafe { MmapFileMut::create(&path) }.unwrap();
    f.truncate(4).unwrap();
    f.write_all(b"data", 0).unwrap();
    f.flush().unwrap();
    f.remove().unwrap();
    assert!(!path.exists());

    // memory + close (hits `_ => Ok(())`)
    let mut f = MmapFileMut::memory_from_vec("close.mem", vec![1, 2]);
    f.close(-1).unwrap();

    // memory + remove (hits `_ => self.deleted = true`)
    let mut f = MmapFileMut::memory_from_vec("rm.mem", vec![3, 4]);
    f.remove().unwrap();
  }

  #[test]
  fn cow_flush_methods_are_noops() {
    // Open a COW-backed disk mapping and exercise every flush variant —
    // each one short-circuits to `Ok(())` via the `if self.is_cow()` guard,
    // covering the previously-unhit cow-branch in `impl_flush!`.
    use scopeguard::defer;
    use std::io::Write;

    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    {
      let mut f = std::fs::File::create(&path).unwrap();
      f.write_all(b"keep me intact").unwrap();
      f.sync_all().unwrap();
    }

    let cow = unsafe { MmapFileMut::open_cow(&path) }.unwrap();
    cow.flush().unwrap();
    cow.flush_async().unwrap();
    cow.flush_range(0, 4).unwrap();
    cow.flush_async_range(0, 4).unwrap();
  }

  #[test]
  fn read_write_error_paths() {
    let mut file = MemoryMmapFileMut::from_vec("e.mem", vec![1, 2, 3, 4]);
    let mut buf = [0u8; 8];

    // reader with offset > len → Err
    assert!(file.reader(99).is_err());
    // writer with offset > len → Err
    assert!(file.writer(99).is_err());

    // read with offset > len → 0
    assert_eq!(file.read(&mut buf, 99), 0);
    // write with offset > len → 0
    assert_eq!(file.write(b"abc", 99), 0);

    // read_exact: offset > len → Err
    assert!(file.read_exact(&mut buf, 99).is_err());
    // read_exact: remaining < dst_len → Err
    assert!(file.read_exact(&mut buf, 0).is_err());

    // write_all: offset > len → Err
    assert!(file.write_all(b"data", 99).is_err());
    // write_all: remaining < src_len → Err
    assert!(file.write_all(&[0u8; 8], 0).is_err());

    // read_u8/i8 with offset > len → Err (None arm)
    assert!(file.read_u8(99).is_err());
    assert!(file.read_i8(99).is_err());
    // read_u8/i8 at offset == len → remaining = 0 < 1 → Err (Some arm)
    assert!(file.read_u8(4).is_err());
    assert!(file.read_i8(4).is_err());
  }
}

#[cfg(feature = "tokio")]
mod axync {
  macro_rules! tokio_async_tests {
    ($([$test_fn: ident, $init: block]), +$(,)?) => {
      use std::io::SeekFrom;
      use scopeguard::defer;
      use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

      const SANITY_TEXT: &'static str = "Hello, async file!";
      const MODIFIED_SANITY_TEXT: &'static str = "Hello, modified async file!";

      $(
      #[cfg(feature = "tokio")]
      #[tokio::test]
      async fn $test_fn() {
        let mut file = $init;
        assert_eq!(file.as_mut_slice().len(), 0);
          file.truncate(8096).await.unwrap(); // 1 KB
        let mut writter = file.writer(0).unwrap();
          AsyncWriteExt::write_all(&mut writter, SANITY_TEXT.as_bytes()).await.unwrap();
          AsyncSeekExt::seek(&mut writter, SeekFrom::Start(100)).await.unwrap();
          AsyncWriteExt::write_i8(&mut writter, -8).await.unwrap();
          AsyncWriteExt::write_i16(&mut writter, -16).await.unwrap();
          AsyncWriteExt::write_i32(&mut writter, -32).await.unwrap();
          AsyncWriteExt::write_i64(&mut writter, -64).await.unwrap();
          writter.flush().await.unwrap();
          writter.seek(SeekFrom::End(0)).await.unwrap();
        let mut reader = file.reader(0).unwrap();
        let mut buf = [0; SANITY_TEXT.len()];
          reader.read_exact(&mut buf).await.unwrap();
        assert!(buf.eq(SANITY_TEXT.as_bytes()));
          AsyncSeekExt::seek(&mut reader, SeekFrom::Start(100)).await.unwrap();
        assert_eq!(-8, AsyncReadExt::read_i8(&mut reader).await.unwrap());
        assert_eq!(-16, AsyncReadExt::read_i16(&mut reader).await.unwrap());
        assert_eq!(-32, AsyncReadExt::read_i32(&mut reader).await.unwrap());
        assert_eq!(-64, AsyncReadExt::read_i64(&mut reader).await.unwrap());

        let mut range_writer = file.range_writer(8000, 96).unwrap();
          AsyncWriteExt::write_u8(&mut range_writer, 8).await.unwrap();
          AsyncWriteExt::write_u16(&mut range_writer, 16).await.unwrap();
          AsyncWriteExt::write_u32(&mut range_writer, 32).await.unwrap();
          AsyncWriteExt::write_u64(&mut range_writer, 64).await.unwrap();
          range_writer.flush().await.unwrap();

        let mut range_reader = file.range_reader(8000, 96).unwrap();
        assert_eq!(8, AsyncReadExt::read_u8(&mut range_reader).await.unwrap());
        assert_eq!(16, AsyncReadExt::read_u16(&mut range_reader).await.unwrap());
        assert_eq!(32, AsyncReadExt::read_u32(&mut range_reader).await.unwrap());
        assert_eq!(64, AsyncReadExt::read_u64(&mut range_reader).await.unwrap());

          file.write_u8(8, 1000).unwrap();
          file.write_u16(16, 1001).unwrap();
          file.write_u32(32, 1003).unwrap();
          file.write_u64(64, 1007).unwrap();
          file.write_u128(128, 1015).unwrap();
          file.write_u16_le(16, 1031).unwrap();
          file.write_u32_le(32, 1033).unwrap();
          file.write_u64_le(64, 1037).unwrap();
          file.write_u128_le(128, 1045).unwrap();
          file.write_usize(64, 1061).unwrap();
          file.write_usize_le(64, 1069).unwrap();

        assert_eq!(8, file.read_u8(1000).unwrap());
        assert_eq!(16, file.read_u16(1001).unwrap());
        assert_eq!(32, file.read_u32(1003).unwrap());
        assert_eq!(64, file.read_u64(1007).unwrap());
        assert_eq!(128, file.read_u128(1015).unwrap());
        assert_eq!(16, file.read_u16_le(1031).unwrap());
        assert_eq!(32, file.read_u32_le(1033).unwrap());
        assert_eq!(64, file.read_u64_le(1037).unwrap());
        assert_eq!(128, file.read_u128_le(1045).unwrap());
        assert_eq!(64, file.read_usize(1061).unwrap());
        assert_eq!(64, file.read_usize_le(1069).unwrap());

          file.write_i8(-8, 2000).unwrap();
          file.write_i16(-16, 2001).unwrap();
          file.write_i32(-32, 2003).unwrap();
          file.write_i64(-64, 2007).unwrap();
          file.write_i128(-128, 2015).unwrap();
          file.write_i16_le(-16, 2031).unwrap();
          file.write_i32_le(-32, 2033).unwrap();
          file.write_i64_le(-64, 2037).unwrap();
          file.write_i128_le(-128, 2045).unwrap();
          file.write_isize(-64, 2061).unwrap();
          file.write_isize_le(-64, 2069).unwrap();

        assert_eq!(-8, file.read_i8(2000).unwrap());
        assert_eq!(-16, file.read_i16(2001).unwrap());
        assert_eq!(-32, file.read_i32(2003).unwrap());
        assert_eq!(-64, file.read_i64(2007).unwrap());
        assert_eq!(-128, file.read_i128(2015).unwrap());
        assert_eq!(-16, file.read_i16_le(2031).unwrap());
        assert_eq!(-32, file.read_i32_le(2033).unwrap());
        assert_eq!(-64, file.read_i64_le(2037).unwrap());
        assert_eq!(-128, file.read_i128_le(2045).unwrap());
        assert_eq!(-64, file.read_isize(2061).unwrap());
        assert_eq!(-64, file.read_isize_le(2069).unwrap());

          file.write_f32(32.0, 3000).unwrap();
          file.write_f32_le(32.0, 3004).unwrap();
          file.write_f64(64.0, 3008).unwrap();
          file.write_f64_le(64.0, 3016).unwrap();
        assert_eq!(32.0, file.read_f32(3000).unwrap());
        assert_eq!(32.0, file.read_f32_le(3004).unwrap());
        assert_eq!(64.0, file.read_f64(3008).unwrap());
        assert_eq!(64.0, file.read_f64_le(3016).unwrap());

          file.zero_range(3000, 3024);

          file.truncate(0).await.unwrap();
          file.truncate(100).await.unwrap();

        let st = file.bytes_mut(0, SANITY_TEXT.len()).unwrap();
          st.copy_from_slice(SANITY_TEXT.as_bytes());

        let n = file.write(MODIFIED_SANITY_TEXT.as_bytes(), 0);
        assert_eq!(n, MODIFIED_SANITY_TEXT.len());

        let mst = file.bytes(0, MODIFIED_SANITY_TEXT.len()).unwrap();
        assert_eq!(mst, MODIFIED_SANITY_TEXT.as_bytes());

        let mut vec = vec![0; MODIFIED_SANITY_TEXT.len()];
        let n = file.read(vec.as_mut_slice(), 0);
        assert_eq!(n, MODIFIED_SANITY_TEXT.len());

        let sm = file.slice_mut(MODIFIED_SANITY_TEXT.len(), 4);
          sm.copy_from_slice(&32u32.to_be_bytes());

        let buf = file.slice(MODIFIED_SANITY_TEXT.len(), 4);
        let n = u32::from_be_bytes(buf.try_into().unwrap());
        assert_eq!(n, 32);

        let v = file.copy_all_to_vec();
        assert_eq!(v.len(), 100);
        assert_eq!(&v[..MODIFIED_SANITY_TEXT.len()], MODIFIED_SANITY_TEXT.as_bytes());
        let v = file.copy_range_to_vec(0, MODIFIED_SANITY_TEXT.len());
        assert_eq!(v.as_slice(), MODIFIED_SANITY_TEXT.as_bytes());

        let pb = get_random_filename();
          file.write_all_to_new_file(&pb).await.unwrap();
        defer!(let _ = std::fs::remove_file(&pb););

        let pb1 = get_random_filename();
        defer!(let _ = std::fs::remove_file(&pb1););
          file.write_range_to_new_file(&pb1, 0, MODIFIED_SANITY_TEXT.len()).await.unwrap();

        let mut file = tokio::fs::File::open(&pb).await.unwrap();
        assert_eq!(file.metadata().await.unwrap().len(), 100);
        let mut buf = vec![0; MODIFIED_SANITY_TEXT.len()];
          file.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf.as_slice(), MODIFIED_SANITY_TEXT.as_bytes());
          drop(file);

        let mut file = tokio::fs::File::open(&pb1).await.unwrap();
        assert_eq!(file.metadata().await.unwrap().len(), MODIFIED_SANITY_TEXT.len() as u64);
        let mut buf = vec![0; MODIFIED_SANITY_TEXT.len()];
          file.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf.as_slice(), MODIFIED_SANITY_TEXT.as_bytes());
          drop(file);
      }
      )*
    }
    }

  use super::*;
  use crate::{
    raw::tokio::{AsyncDiskMmapFileMut, AsyncMemoryMmapFileMut},
    tokio::{AsyncMmapFileExt, AsyncMmapFileMut, AsyncMmapFileMutExt},
  };

  tokio_async_tests!(
    [test_async_memory_file_mut, {
      AsyncMemoryMmapFileMut::new("memory.txt")
    }],
    [test_async_mmap_file_mut, {
      let mut file = AsyncMmapFileMut::from(
        unsafe { AsyncDiskMmapFileMut::create(get_random_filename()) }
          .await
          .unwrap(),
      );
      file.set_remove_on_drop(true);
      assert!(file.get_remove_on_drop());
      file
    }],
  );

  #[tokio::test]
  async fn async_read_returns_partial_count_at_eof() {
    let file = AsyncMemoryMmapFileMut::from_vec("memory.txt", vec![1, 2, 3, 4]);
    let mut dst = [0; 8];

    let n = file.read(&mut dst, 2);

    assert_eq!(n, 2);
    assert_eq!(&dst[..2], &[3, 4]);
    assert_eq!(&dst[2..], &[0; 6]);
  }

  #[tokio::test]
  async fn async_checked_range_methods_reject_overflow() {
    let mut file = AsyncMemoryMmapFileMut::from_vec("memory.txt", vec![1, 2, 3, 4]);
    let path = get_random_filename();

    assert!(file.bytes(usize::MAX, 1).is_err());
    assert!(file.range_reader(usize::MAX, 1).is_err());
    assert!(file
      .write_range_to_new_file(&path, usize::MAX, 1)
      .await
      .is_err());
    assert!(file.bytes_mut(usize::MAX, 1).is_err());
    assert!(file.range_writer(usize::MAX, 1).is_err());
  }

  #[tokio::test]
  async fn async_bytes_mut_allows_exact_fit() {
    let mut file = AsyncMemoryMmapFileMut::from_vec("memory.txt", vec![1, 2, 3, 4]);

    assert_eq!(file.bytes_mut(0, 4).unwrap().len(), 4);
    assert_eq!(file.bytes_mut(4, 0).unwrap().len(), 0);
    assert_eq!(file.writer(4).unwrap().len(), 0);
  }

  // -- coverage: async memory file constructors --------------------------

  use crate::{raw::tokio::AsyncMemoryMmapFile, tokio::AsyncMmapFile, MetaDataExt};
  use bytes::Bytes;
  use std::path::Path;

  #[tokio::test]
  async fn async_memory_mmap_file_constructors() {
    let f = AsyncMemoryMmapFile::new("a.mem", Bytes::from_static(b"abc"));
    assert_eq!(f.path(), Path::new("a.mem"));
    assert_eq!(f.as_slice(), b"abc");

    let f = AsyncMemoryMmapFile::from_vec("b.mem", vec![1, 2, 3]);
    assert_eq!(f.as_slice(), &[1, 2, 3]);

    let f = AsyncMemoryMmapFile::from_string("c.mem", "hello".to_string());
    assert_eq!(f.as_slice(), b"hello");

    let f = AsyncMemoryMmapFile::from_slice("d.mem", b"world");
    assert_eq!(f.as_slice(), b"world");

    let f = AsyncMemoryMmapFile::from_str("e.mem", "static");
    assert_eq!(f.as_slice(), b"static");

    let f = AsyncMemoryMmapFile::copy_from_slice("f.mem", b"copy");
    assert_eq!(f.as_slice(), b"copy");

    let bytes = f.into_bytes();
    assert_eq!(bytes.as_ref(), b"copy");
  }

  #[tokio::test]
  async fn async_memory_mmap_file_mut_constructors() {
    let f = AsyncMemoryMmapFileMut::new("new.mem");
    assert_eq!(f.path(), Path::new("new.mem"));

    let f = AsyncMemoryMmapFileMut::with_capacity("cap.mem", 100);
    assert_eq!(f.path(), Path::new("cap.mem"));

    let f = AsyncMemoryMmapFileMut::from_vec("v.mem", vec![1, 2, 3]);
    assert_eq!(f.as_slice(), &[1, 2, 3]);

    let f = AsyncMemoryMmapFileMut::from_string("s.mem", "data".to_string());
    assert_eq!(f.as_slice(), b"data");

    let f = AsyncMemoryMmapFileMut::from_str("st.mem", "static");
    assert_eq!(f.as_slice(), b"static");

    let f = AsyncMemoryMmapFileMut::from_slice("sl.mem", b"slice");
    assert_eq!(f.as_slice(), b"slice");

    let frozen = f.freeze();
    assert_eq!(frozen.as_slice(), b"slice");

    let f = AsyncMemoryMmapFileMut::from_vec("foo.mem", vec![9, 8, 7]);
    let bm = f.into_bytes_mut();
    assert_eq!(bm.as_ref(), &[9, 8, 7]);
  }

  #[tokio::test]
  async fn async_mmap_file_memory_constructors() {
    let f = AsyncMmapFile::memory("a", Bytes::from_static(b"abc"));
    assert_eq!(f.as_slice(), b"abc");

    let f = AsyncMmapFile::memory_from_vec("b", vec![1, 2]);
    assert_eq!(f.as_slice(), &[1, 2]);

    let f = AsyncMmapFile::memory_from_string("c", "hi".to_string());
    assert_eq!(f.as_slice(), b"hi");

    let f = AsyncMmapFile::memory_from_slice("d", b"static");
    assert_eq!(f.as_slice(), b"static");

    let f = AsyncMmapFile::memory_from_str("e", "str");
    assert_eq!(f.as_slice(), b"str");

    let f = AsyncMmapFile::memory_copy_from_slice("g", b"copy");
    assert_eq!(f.as_slice(), b"copy");
  }

  #[tokio::test]
  async fn async_mmap_file_mut_memory_constructors() {
    let f = AsyncMmapFileMut::memory("a");
    assert_eq!(f.as_slice().len(), 0);

    let f = AsyncMmapFileMut::memory_with_capacity("b", 100);
    assert_eq!(f.path(), Path::new("b"));

    let f = AsyncMmapFileMut::memory_from_vec("c", vec![1, 2]);
    assert_eq!(f.as_slice(), &[1, 2]);

    let f = AsyncMmapFileMut::memory_from_string("d", "hi".to_string());
    assert_eq!(f.as_slice(), b"hi");

    let f = AsyncMmapFileMut::memory_from_slice("e", b"static");
    assert_eq!(f.as_slice(), b"static");

    let f = AsyncMmapFileMut::memory_from_str("f", "str");
    assert_eq!(f.as_slice(), b"str");

    // Exercise Memory arm of AsyncMmapFileMutInner dispatchers.
    let mut f = AsyncMmapFileMut::memory_from_vec("disp.mem", vec![1, 2, 3]);
    let _ = f.as_mut_slice();
    assert!(!f.is_cow());
    f.flush().unwrap();
    f.flush_async().unwrap();
    f.flush_range(0, 1).unwrap();
    f.flush_async_range(0, 1).unwrap();
    assert_eq!(AsyncMmapFileExt::len(&f), 3);
  }

  #[tokio::test]
  async fn async_memory_variant_lock_methods_are_noops() {
    let mut f = AsyncMmapFileMut::memory_from_vec("lock.mem", vec![1, 2, 3]);
    f.lock().unwrap();
    unsafe { f.lock_shared().unwrap() };
    f.try_lock().unwrap();
    unsafe { f.try_lock_shared().unwrap() };
    unsafe { f.unlock().unwrap() };

    let mut f = AsyncMmapFile::memory_from_vec("rlock.mem", vec![4, 5]);
    f.lock().unwrap();
    unsafe { f.lock_shared().unwrap() };
    f.try_lock().unwrap();
    unsafe { f.try_lock_shared().unwrap() };
    unsafe { f.unlock().unwrap() };
  }

  #[tokio::test]
  async fn async_memory_file_metadata_and_accessors() {
    let f = AsyncMemoryMmapFile::from_str("meta.mem", "data");
    let meta = f.metadata().await.unwrap();
    assert_eq!(meta.len(), 4);
    assert_eq!(AsyncMmapFileExt::len(&f), 4);

    let f = AsyncMemoryMmapFileMut::from_str("metam.mem", "data");
    let meta = f.metadata().await.unwrap();
    assert_eq!(meta.len(), 4);
    assert!(!f.is_exec());
    assert_eq!(AsyncMmapFileExt::len(&f), 4);
    assert!(!AsyncMmapFileMutExt::is_cow(&f));

    // into_bytes / drop_remove / close_with_truncate on the Mut variant —
    // each is a no-op trait impl that previously had no caller.
    let f = AsyncMemoryMmapFileMut::from_str("ib.mem", "abc");
    let b = f.into_bytes();
    assert_eq!(b.as_ref(), b"abc");

    let f = AsyncMemoryMmapFileMut::from_str("dr.mem", "abc");
    AsyncMmapFileMutExt::drop_remove(f).await.unwrap();

    let f = AsyncMemoryMmapFileMut::from_str("cw.mem", "abc");
    AsyncMmapFileMutExt::close_with_truncate(f, 1)
      .await
      .unwrap();
  }

  #[tokio::test]
  async fn async_options_builder_branches() {
    use crate::tokio::AsyncOptions;

    let _ = AsyncOptions::new()
      .max_size(1024)
      .read(true)
      .write(true)
      .create(false)
      .create_new(false)
      .append(false)
      .truncate(false)
      .offset(0)
      .len(64)
      .populate()
      .stack();

    let _ = AsyncOptions::default();

    #[cfg(unix)]
    {
      let _ = AsyncOptions::new().mode(0o644).custom_flags(0);
    }
  }

  #[tokio::test]
  async fn async_memory_variant_freeze_and_close_and_remove() {
    let f = AsyncMmapFileMut::memory_from_vec("freeze.mem", vec![1, 2, 3]);
    let frozen = f.freeze().unwrap();
    assert_eq!(frozen.as_slice(), &[1, 2, 3]);

    let mut f = AsyncMmapFileMut::memory_from_vec("close.mem", vec![4, 5]);
    f.close(-1).await.unwrap();

    let mut f = AsyncMmapFileMut::memory_from_vec("rm.mem", vec![6, 7]);
    f.remove().await.unwrap();
  }

  #[tokio::test]
  async fn tokio_utils_helpers_smoke() {
    use crate::utils::tokio::*;
    use scopeguard::defer;

    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    {
      let _ = create_file_async(&path).await.unwrap();
    }

    let _ = open_read_only_file_async(&path).await.unwrap();
    let _ = open_exist_file_async(&path).await.unwrap();
    let _ = open_exist_file_with_append_async(&path).await.unwrap();
    let _ = open_or_create_file_async(&path).await.unwrap();
    let _ = open_file_with_truncate_async(&path).await.unwrap();
    sync_dir_async(path.parent().unwrap()).await.unwrap();
    sync_parent_async(&path).await.unwrap();
    assert!(sync_dir_async(&path).await.is_err());
  }

  #[tokio::test]
  async fn async_disk_wrapper_lock_method_dispatch() {
    use scopeguard::defer;
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    let mut f = unsafe { AsyncMmapFileMut::create(&path).await }.unwrap();
    let _ = f.lock();
    let _ = unsafe { f.lock_shared() };
    let _ = f.try_lock();
    let _ = unsafe { f.try_lock_shared() };
    let _ = unsafe { f.unlock() };
  }

  #[tokio::test]
  async fn async_inherent_close_and_remove() {
    use scopeguard::defer;

    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let mut f = unsafe { AsyncMmapFileMut::create(&path).await }.unwrap();
    f.truncate(8).await.unwrap();
    AsyncMmapFileMutExt::write_all(&mut f, b"abcdefgh", 0).unwrap();
    f.flush().unwrap();
    f.close(4).await.unwrap();
    // After close, methods route through Empty arm of every dispatcher.
    let _ = f.len();
    let _ = f.as_slice();
    let _ = f.path();
    let _ = f.is_exec();
    let _ = f.metadata().await;
    let _ = f.lock();
    let _ = unsafe { f.lock_shared() };
    let _ = f.try_lock();
    let _ = unsafe { f.try_lock_shared() };
    let _ = unsafe { f.unlock() };

    let path = get_random_filename();
    let mut f = unsafe { AsyncMmapFileMut::create(&path).await }.unwrap();
    f.truncate(4).await.unwrap();
    f.flush().unwrap();
    f.remove().await.unwrap();
    assert!(!path.exists());
  }

  /// Codex round 15 (high) regression: async open_with_options used to
  /// truncate before validating the mapping range. Verify a tokio caller
  /// cannot lose existing file content when options are invalid.
  #[tokio::test]
  async fn async_invalid_options_with_truncate_preserve_existing_file() {
    use crate::tokio::AsyncOptions;
    use scopeguard::defer;
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    {
      let mut f = std::fs::File::create(&path).unwrap();
      std::io::Write::write_all(&mut f, b"PRESERVE_ME").unwrap();
      f.sync_all().unwrap();
    }

    let opts = AsyncOptions::new().truncate(true).offset(1).len(2);
    let result =
      unsafe { AsyncMmapFileMut::open_with_options(&path, opts).await }.map(|_| "should reject");
    let err = result.expect_err("invalid offset/len must reject");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(std::fs::read(&path).unwrap(), b"PRESERVE_ME");
  }

  #[tokio::test]
  async fn async_reader_poll_methods_exercised() {
    use crate::raw::tokio::AsyncMemoryMmapFileMut;
    use tokio::io::{AsyncBufReadExt, AsyncSeekExt as _, AsyncWriteExt};

    let mut file = AsyncMemoryMmapFileMut::from_vec("rdr.mem", vec![1; 256]);

    // Exercise the reader's AsyncBufRead poll_fill_buf/consume. Block
    // scope drops `r` (avoiding clippy::drop_non_drop on the explicit
    // `drop(r)` since the reader doesn't impl Drop) before the mutable
    // borrow below.
    {
      let mut r = file.range_reader(0, 256).unwrap();
      let _ = r.fill_buf().await.unwrap();
      r.consume(4);
    }

    // Exercise the writer's poll_seek + poll_write + poll_flush + poll_close.
    let mut w = file.writer(0).unwrap();
    AsyncWriteExt::write_all(&mut w, b"data").await.unwrap();
    AsyncWriteExt::flush(&mut w).await.unwrap();
    let _ = w.seek(std::io::SeekFrom::Start(0)).await.unwrap();
    AsyncWriteExt::shutdown(&mut w).await.unwrap();
  }

  #[tokio::test]
  async fn async_cow_flush_methods_are_noops() {
    use scopeguard::defer;
    use std::io::Write;

    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    {
      let mut f = std::fs::File::create(&path).unwrap();
      f.write_all(b"keep me intact").unwrap();
      f.sync_all().unwrap();
    }

    let cow = unsafe { AsyncMmapFileMut::open_cow(&path).await }.unwrap();
    cow.flush().unwrap();
    cow.flush_async().unwrap();
    cow.flush_range(0, 4).unwrap();
    cow.flush_async_range(0, 4).unwrap();
  }

  #[tokio::test]
  async fn async_read_write_error_paths() {
    let mut file = AsyncMemoryMmapFileMut::from_vec("e.mem", vec![1, 2, 3, 4]);
    let mut buf = [0u8; 8];

    assert!(file.reader(99).is_err());
    assert!(file.writer(99).is_err());
    assert_eq!(file.read(&mut buf, 99), 0);
    assert_eq!(file.write(b"abc", 99), 0);
    assert!(file.read_exact(&mut buf, 99).is_err());
    assert!(file.read_exact(&mut buf, 0).is_err());
    assert!(AsyncMmapFileMutExt::write_all(&mut file, b"data", 99).is_err());
    assert!(AsyncMmapFileMutExt::write_all(&mut file, &[0u8; 8], 0).is_err());
    assert!(file.read_u8(99).is_err());
    assert!(file.read_i8(99).is_err());
  }

  #[tokio::test]
  async fn async_cow_close_does_not_truncate_backing_file() {
    use scopeguard::defer;
    use std::io::Write;

    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    {
      let mut f = std::fs::File::create(&path).unwrap();
      f.write_all(b"keep me intact").unwrap();
      f.sync_all().unwrap();
    }

    {
      let mut cow = unsafe { AsyncMmapFileMut::open_cow(&path).await }.unwrap();
      let err = cow.close(0).await.unwrap_err();
      assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"keep me intact");

    {
      let mut cow = unsafe { AsyncMmapFileMut::open_cow(&path).await }.unwrap();
      cow.close(-1).await.unwrap();
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"keep me intact");

    {
      let cow = unsafe { AsyncMmapFileMut::open_cow(&path).await }.unwrap();
      let err = cow.close_with_truncate(0).await.unwrap_err();
      assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"keep me intact");
  }

  #[tokio::test]
  async fn async_disk_read_only_wrapper_methods() {
    use scopeguard::defer;
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    {
      let mut f = unsafe { AsyncMmapFileMut::create(&path).await }.unwrap();
      f.truncate(8).await.unwrap();
      AsyncMmapFileMutExt::write_all(&mut f, b"hello!!!", 0).unwrap();
      f.flush().unwrap();
    }

    let mut f = unsafe { AsyncMmapFile::open(&path).await }.unwrap();
    assert_eq!(f.as_slice(), b"hello!!!");
    assert_eq!(AsyncMmapFileExt::len(&f), 8);
    assert_eq!(f.path(), path.as_path());
    assert!(!f.is_exec());
    let _ = f.metadata().await.unwrap();
    let _ = f.lock();
    let _ = unsafe { f.lock_shared() };
    let _ = f.try_lock();
    let _ = unsafe { f.try_lock_shared() };
    let _ = unsafe { f.unlock() };
    drop(f);

    use crate::raw::tokio::AsyncDiskMmapFile;
    let mut raw = unsafe { AsyncDiskMmapFile::open(&path).await }.unwrap();
    assert_eq!(AsyncMmapFileExt::len(&raw), 8);
    assert_eq!(AsyncMmapFileExt::as_slice(&raw), b"hello!!!");
    assert_eq!(AsyncMmapFileExt::path(&raw), path.as_path());
    assert!(!AsyncMmapFileExt::is_exec(&raw));
    let _ = AsyncMmapFileExt::metadata(&raw).await.unwrap();
    let _ = AsyncMmapFileExt::lock(&mut raw);
    let _ = unsafe { AsyncMmapFileExt::lock_shared(&mut raw) };
    let _ = AsyncMmapFileExt::try_lock(&mut raw);
    let _ = unsafe { AsyncMmapFileExt::try_lock_shared(&mut raw) };
    let _ = unsafe { AsyncMmapFileExt::unlock(&mut raw) };
  }
}

#[cfg(feature = "smol")]
mod smol_tests {
  use super::*;
  use crate::{
    raw::smol::{AsyncMemoryMmapFile, AsyncMemoryMmapFileMut},
    smol::{AsyncMmapFile, AsyncMmapFileExt, AsyncMmapFileMut, AsyncMmapFileMutExt},
    MetaDataExt,
  };
  use bytes::Bytes;
  use std::path::Path;

  #[smol_potat::test]
  async fn smol_memory_mmap_file_constructors() {
    let f = AsyncMemoryMmapFile::new("a.mem", Bytes::from_static(b"abc"));
    assert_eq!(f.path(), Path::new("a.mem"));
    assert_eq!(f.as_slice(), b"abc");

    let f = AsyncMemoryMmapFile::from_vec("b.mem", vec![1, 2, 3]);
    assert_eq!(f.as_slice(), &[1, 2, 3]);

    let f = AsyncMemoryMmapFile::from_string("c.mem", "hello".to_string());
    assert_eq!(f.as_slice(), b"hello");

    let f = AsyncMemoryMmapFile::from_slice("d.mem", b"world");
    assert_eq!(f.as_slice(), b"world");

    let f = AsyncMemoryMmapFile::from_str("e.mem", "static");
    assert_eq!(f.as_slice(), b"static");

    let f = AsyncMemoryMmapFile::copy_from_slice("f.mem", b"copy");
    assert_eq!(f.as_slice(), b"copy");

    let bytes = f.into_bytes();
    assert_eq!(bytes.as_ref(), b"copy");
  }

  #[smol_potat::test]
  async fn smol_memory_mmap_file_mut_constructors() {
    let f = AsyncMemoryMmapFileMut::new("new.mem");
    assert_eq!(f.path(), Path::new("new.mem"));

    let f = AsyncMemoryMmapFileMut::with_capacity("cap.mem", 100);
    assert_eq!(f.path(), Path::new("cap.mem"));

    let f = AsyncMemoryMmapFileMut::from_vec("v.mem", vec![1, 2, 3]);
    assert_eq!(f.as_slice(), &[1, 2, 3]);

    let f = AsyncMemoryMmapFileMut::from_string("s.mem", "data".to_string());
    assert_eq!(f.as_slice(), b"data");

    let f = AsyncMemoryMmapFileMut::from_str("st.mem", "static");
    assert_eq!(f.as_slice(), b"static");

    let f = AsyncMemoryMmapFileMut::from_slice("sl.mem", b"slice");
    assert_eq!(f.as_slice(), b"slice");

    let frozen = f.freeze();
    assert_eq!(frozen.as_slice(), b"slice");
  }

  #[smol_potat::test]
  async fn smol_mmap_file_memory_constructors() {
    let f = AsyncMmapFile::memory("a", Bytes::from_static(b"abc"));
    assert_eq!(f.as_slice(), b"abc");

    let f = AsyncMmapFile::memory_from_vec("b", vec![1, 2]);
    assert_eq!(f.as_slice(), &[1, 2]);

    let f = AsyncMmapFile::memory_from_string("c", "hi".to_string());
    assert_eq!(f.as_slice(), b"hi");

    let f = AsyncMmapFile::memory_from_slice("d", b"static");
    assert_eq!(f.as_slice(), b"static");

    let f = AsyncMmapFile::memory_from_str("e", "str");
    assert_eq!(f.as_slice(), b"str");

    let f = AsyncMmapFile::memory_copy_from_slice("g", b"copy");
    assert_eq!(f.as_slice(), b"copy");
  }

  #[smol_potat::test]
  async fn smol_mmap_file_mut_memory_constructors() {
    let f = AsyncMmapFileMut::memory("a");
    assert_eq!(f.as_slice().len(), 0);

    let f = AsyncMmapFileMut::memory_with_capacity("b", 100);
    assert_eq!(f.path(), Path::new("b"));

    let f = AsyncMmapFileMut::memory_from_vec("c", vec![1, 2]);
    assert_eq!(f.as_slice(), &[1, 2]);

    let f = AsyncMmapFileMut::memory_from_string("d", "hi".to_string());
    assert_eq!(f.as_slice(), b"hi");

    let f = AsyncMmapFileMut::memory_from_slice("e", b"static");
    assert_eq!(f.as_slice(), b"static");

    let f = AsyncMmapFileMut::memory_from_str("f", "str");
    assert_eq!(f.as_slice(), b"str");

    // Exercise Memory arm of AsyncMmapFileMutInner dispatchers.
    let mut f = AsyncMmapFileMut::memory_from_vec("disp.mem", vec![1, 2, 3]);
    let _ = f.as_mut_slice();
    assert!(!f.is_cow());
    f.flush().unwrap();
    f.flush_async().unwrap();
    f.flush_range(0, 1).unwrap();
    f.flush_async_range(0, 1).unwrap();
    assert_eq!(AsyncMmapFileExt::len(&f), 3);
  }

  #[smol_potat::test]
  async fn smol_memory_variant_lock_methods_are_noops() {
    let mut f = AsyncMmapFileMut::memory_from_vec("lock.mem", vec![1, 2, 3]);
    f.lock().unwrap();
    unsafe { f.lock_shared().unwrap() };
    f.try_lock().unwrap();
    unsafe { f.try_lock_shared().unwrap() };
    unsafe { f.unlock().unwrap() };

    let mut f = AsyncMmapFile::memory_from_vec("rlock.mem", vec![4, 5]);
    f.lock().unwrap();
    unsafe { f.lock_shared().unwrap() };
    f.try_lock().unwrap();
    unsafe { f.try_lock_shared().unwrap() };
    unsafe { f.unlock().unwrap() };
  }

  #[smol_potat::test]
  async fn smol_memory_file_metadata_and_accessors() {
    let f = AsyncMemoryMmapFile::from_str("meta.mem", "data");
    let meta = f.metadata().await.unwrap();
    assert_eq!(meta.len(), 4);
    assert_eq!(AsyncMmapFileExt::len(&f), 4);

    let f = AsyncMemoryMmapFileMut::from_str("metam.mem", "data");
    let meta = f.metadata().await.unwrap();
    assert_eq!(meta.len(), 4);
    assert!(!f.is_exec());
    assert_eq!(AsyncMmapFileExt::len(&f), 4);
    assert!(!AsyncMmapFileMutExt::is_cow(&f));

    let f = AsyncMemoryMmapFileMut::from_str("ib.mem", "abc");
    let b = f.into_bytes();
    assert_eq!(b.as_ref(), b"abc");

    let f = AsyncMemoryMmapFileMut::from_str("dr.mem", "abc");
    AsyncMmapFileMutExt::drop_remove(f).await.unwrap();

    let f = AsyncMemoryMmapFileMut::from_str("cw.mem", "abc");
    AsyncMmapFileMutExt::close_with_truncate(f, 1)
      .await
      .unwrap();
  }

  #[smol_potat::test]
  async fn smol_options_builder_branches() {
    use crate::smol::AsyncOptions;

    let _ = AsyncOptions::new()
      .max_size(1024)
      .read(true)
      .write(true)
      .create(false)
      .create_new(false)
      .append(false)
      .truncate(false)
      .offset(0)
      .len(64)
      .populate()
      .stack();

    let _ = AsyncOptions::default();

    #[cfg(unix)]
    {
      let _ = AsyncOptions::new().mode(0o644).custom_flags(0);
    }
  }

  #[smol_potat::test]
  async fn smol_memory_variant_freeze_and_close_and_remove() {
    let f = AsyncMmapFileMut::memory_from_vec("freeze.mem", vec![1, 2, 3]);
    let frozen = f.freeze().unwrap();
    assert_eq!(frozen.as_slice(), &[1, 2, 3]);

    let mut f = AsyncMmapFileMut::memory_from_vec("close.mem", vec![4, 5]);
    f.close(-1).await.unwrap();

    let mut f = AsyncMmapFileMut::memory_from_vec("rm.mem", vec![6, 7]);
    f.remove().await.unwrap();
  }

  #[smol_potat::test]
  async fn smol_utils_helpers_smoke() {
    use crate::utils::smol::*;
    use scopeguard::defer;

    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    {
      let _ = create_file_async(&path).await.unwrap();
    }

    let _ = open_read_only_file_async(&path).await.unwrap();
    let _ = open_exist_file_async(&path).await.unwrap();
    let _ = open_exist_file_with_append_async(&path).await.unwrap();
    let _ = open_or_create_file_async(&path).await.unwrap();
    let _ = open_file_with_truncate_async(&path).await.unwrap();
    sync_dir_async(path.parent().unwrap()).await.unwrap();
    sync_parent_async(&path).await.unwrap();
    assert!(sync_dir_async(&path).await.is_err());
  }

  #[smol_potat::test]
  async fn smol_disk_wrapper_lock_method_dispatch() {
    use crate::raw::smol::AsyncDiskMmapFileMut;
    use scopeguard::defer;
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    let mut f = AsyncMmapFileMut::from(
      unsafe { AsyncDiskMmapFileMut::create(&path) }
        .await
        .unwrap(),
    );
    let _ = f.lock();
    let _ = unsafe { f.lock_shared() };
    let _ = f.try_lock();
    let _ = unsafe { f.try_lock_shared() };
    let _ = unsafe { f.unlock() };
  }

  /// Codex round 15 (high) regression: see tokio counterpart. Validates
  /// the smol async path also rejects invalid options before destructive
  /// truncation.
  #[smol_potat::test]
  async fn smol_invalid_options_with_truncate_preserve_existing_file() {
    use crate::smol::AsyncOptions;
    use scopeguard::defer;
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    {
      let mut f = std::fs::File::create(&path).unwrap();
      std::io::Write::write_all(&mut f, b"PRESERVE_ME").unwrap();
      f.sync_all().unwrap();
    }
    let opts = AsyncOptions::new().truncate(true).offset(1).len(2);
    let result =
      unsafe { AsyncMmapFileMut::open_with_options(&path, opts).await }.map(|_| "should reject");
    let err = result.expect_err("invalid offset/len must reject");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(std::fs::read(&path).unwrap(), b"PRESERVE_ME");
  }

  #[smol_potat::test]
  async fn smol_inherent_close_and_remove() {
    use crate::raw::smol::AsyncDiskMmapFileMut;
    use scopeguard::defer;

    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    let mut f = AsyncMmapFileMut::from(
      unsafe { AsyncDiskMmapFileMut::create(&path) }
        .await
        .unwrap(),
    );
    f.truncate(8).await.unwrap();
    AsyncMmapFileMutExt::write_all(&mut f, b"abcdefgh", 0).unwrap();
    f.flush().unwrap();
    f.close(4).await.unwrap();
    let _ = f.len();
    let _ = f.as_slice();
    let _ = f.path();
    let _ = f.is_exec();
    let _ = f.metadata().await;
    let _ = f.lock();
    let _ = unsafe { f.lock_shared() };
    let _ = f.try_lock();
    let _ = unsafe { f.try_lock_shared() };
    let _ = unsafe { f.unlock() };

    let path = get_random_filename();
    let mut f = AsyncMmapFileMut::from(
      unsafe { AsyncDiskMmapFileMut::create(&path) }
        .await
        .unwrap(),
    );
    f.truncate(4).await.unwrap();
    f.flush().unwrap();
    f.remove().await.unwrap();
    assert!(!path.exists());
  }

  #[smol_potat::test]
  async fn smol_reader_poll_methods_exercised() {
    use crate::raw::smol::AsyncMemoryMmapFileMut;
    use smol::io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt};

    let mut file = AsyncMemoryMmapFileMut::from_vec("rdr.mem", vec![1; 256]);

    // Exercise the reader's AsyncBufRead poll_fill_buf/consume. Use a
    // block to drop `r` (without `drop(r)` which clippy::drop_non_drop
    // flags because `AsyncMmapFileReader` doesn't impl Drop) before the
    // mutable borrow that follows.
    {
      let mut r = file.range_reader(0, 256).unwrap();
      let _ = AsyncBufReadExt::fill_buf(&mut r).await.unwrap();
      AsyncBufReadExt::consume(&mut r, 4);
    }

    // Exercise writer's poll_seek + poll_write + poll_flush + poll_close.
    let mut w = file.writer(0).unwrap();
    AsyncWriteExt::write_all(&mut w, b"data").await.unwrap();
    AsyncWriteExt::flush(&mut w).await.unwrap();
    let _ = AsyncSeekExt::seek(&mut w, std::io::SeekFrom::Start(0))
      .await
      .unwrap();
    AsyncWriteExt::close(&mut w).await.unwrap();
  }

  #[smol_potat::test]
  async fn smol_cow_flush_methods_are_noops() {
    use scopeguard::defer;
    use std::io::Write;

    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    {
      let mut f = std::fs::File::create(&path).unwrap();
      f.write_all(b"keep me intact").unwrap();
      f.sync_all().unwrap();
    }

    let cow = unsafe { AsyncMmapFileMut::open_cow(&path).await }.unwrap();
    cow.flush().unwrap();
    cow.flush_async().unwrap();
    cow.flush_range(0, 4).unwrap();
    cow.flush_async_range(0, 4).unwrap();
  }

  #[smol_potat::test]
  async fn smol_read_write_error_paths() {
    let mut file = AsyncMemoryMmapFileMut::from_vec("e.mem", vec![1, 2, 3, 4]);
    let mut buf = [0u8; 8];

    assert!(file.reader(99).is_err());
    assert!(file.writer(99).is_err());
    assert_eq!(file.read(&mut buf, 99), 0);
    assert_eq!(file.write(b"abc", 99), 0);
    assert!(file.read_exact(&mut buf, 99).is_err());
    assert!(file.read_exact(&mut buf, 0).is_err());
    assert!(AsyncMmapFileMutExt::write_all(&mut file, b"data", 99).is_err());
    assert!(AsyncMmapFileMutExt::write_all(&mut file, &[0u8; 8], 0).is_err());
    assert!(file.read_u8(99).is_err());
    assert!(file.read_i8(99).is_err());
  }

  #[smol_potat::test]
  async fn smol_cow_close_does_not_truncate_backing_file() {
    use scopeguard::defer;
    use std::io::Write;

    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););
    {
      let mut f = std::fs::File::create(&path).unwrap();
      f.write_all(b"keep me intact").unwrap();
      f.sync_all().unwrap();
    }

    {
      let mut cow = unsafe { AsyncMmapFileMut::open_cow(&path).await }.unwrap();
      let err = cow.close(0).await.unwrap_err();
      assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"keep me intact");

    {
      let mut cow = unsafe { AsyncMmapFileMut::open_cow(&path).await }.unwrap();
      cow.close(-1).await.unwrap();
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"keep me intact");

    {
      let cow = unsafe { AsyncMmapFileMut::open_cow(&path).await }.unwrap();
      let err = cow.close_with_truncate(0).await.unwrap_err();
      assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"keep me intact");
  }

  #[smol_potat::test]
  async fn smol_disk_read_only_wrapper_methods() {
    use crate::raw::smol::AsyncDiskMmapFileMut;
    use scopeguard::defer;
    let path = get_random_filename();
    defer!(let _ = std::fs::remove_file(&path););

    {
      let mut f = AsyncMmapFileMut::from(
        unsafe { AsyncDiskMmapFileMut::create(&path) }
          .await
          .unwrap(),
      );
      f.truncate(8).await.unwrap();
      AsyncMmapFileMutExt::write_all(&mut f, b"hello!!!", 0).unwrap();
      f.flush().unwrap();
    }

    let mut f = unsafe { AsyncMmapFile::open(&path).await }.unwrap();
    assert_eq!(f.as_slice(), b"hello!!!");
    assert_eq!(AsyncMmapFileExt::len(&f), 8);
    assert_eq!(f.path(), path.as_path());
    assert!(!f.is_exec());
    let _ = f.metadata().await.unwrap();
    let _ = f.lock();
    let _ = unsafe { f.lock_shared() };
    let _ = f.try_lock();
    let _ = unsafe { f.try_lock_shared() };
    let _ = unsafe { f.unlock() };
    drop(f);

    use crate::raw::smol::AsyncDiskMmapFile;
    let mut raw = unsafe { AsyncDiskMmapFile::open(&path).await }.unwrap();
    assert_eq!(AsyncMmapFileExt::len(&raw), 8);
    assert_eq!(AsyncMmapFileExt::as_slice(&raw), b"hello!!!");
    assert_eq!(AsyncMmapFileExt::path(&raw), path.as_path());
    assert!(!AsyncMmapFileExt::is_exec(&raw));
    let _ = AsyncMmapFileExt::metadata(&raw).await.unwrap();
    let _ = AsyncMmapFileExt::lock(&mut raw);
    let _ = unsafe { AsyncMmapFileExt::lock_shared(&mut raw) };
    let _ = AsyncMmapFileExt::try_lock(&mut raw);
    let _ = unsafe { AsyncMmapFileExt::try_lock_shared(&mut raw) };
    let _ = unsafe { AsyncMmapFileExt::unlock(&mut raw) };
  }
}
