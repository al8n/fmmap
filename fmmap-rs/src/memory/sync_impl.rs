use std::path::{Path, PathBuf};
use std::time::SystemTime;
use bytes::{Bytes, BytesMut};
use crate::{MmapFileExt, MmapFileMutExt, MetaData};
use crate::metadata::MemoryMetaData;

/// Use [`Bytes`] to mock a mmap, which is useful for test and in-memory storage engine.
///
/// [`Bytes`]: https://docs.rs/bytes/1.1.0/bytes/struct.Bytes.html
pub struct MemoryMmapFile {
    mmap: Bytes,
    path: PathBuf,
    create_at: SystemTime,
}

impl_mmap_file_ext!(MemoryMmapFile);

impl MemoryMmapFile {
    pub fn new<P: AsRef<Path>>(path: P, data: Bytes) -> Self {
        Self {
            mmap: data,
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now(),
        }
    }

    pub fn from_vec<P: AsRef<Path>>(path: P, src: Vec<u8>) -> Self {
        Self {
            mmap: Bytes::from(src),
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now(),
        }
    }

    pub fn from_string<P: AsRef<Path>>(path: P, src: String) -> Self {
        Self {
            mmap: Bytes::from(src),
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now()
        }
    }

    pub fn from_str<P: AsRef<Path>>(path: P, src: &'static str) -> Self {
        Self {
            mmap: Bytes::from(src),
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now()
        }
    }

    pub fn from_slice<P: AsRef<Path>>(path: P, src: &'static [u8]) -> Self {
        Self {
            mmap: Bytes::from(src),
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now()
        }
    }

    pub fn copy_from_slice<P: AsRef<Path>>(path: P, src: &[u8]) -> Self {
        Self {
            mmap: Bytes::copy_from_slice(src),
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now(),
        }
    }

    pub fn into_bytes(self) -> Bytes {
        self.mmap
    }
}

/// Use [`BytesMut`] to mock a mmap, which is useful for test and in-memory storage engine.
///
/// # Notes
/// MemoryMmapFileMut mocks a mmap behaviour, which means when writing to it,
/// it will not auto-grow its size, so if you want to grow the size of the MemoryMmapFileMut,
/// you need to [`truncate`] it first.
///
/// If you want the auto-grow functionality, please use [`BytesMut`].
///
/// [`truncate`]: structs.MemoryMmapFileMut.html#methods.truncate
/// [`BytesMut`]: https://docs.rs/bytes/1.1.0/bytes/struct.BytesMut.html
pub struct MemoryMmapFileMut {
    mmap: BytesMut,
    path: PathBuf,
    create_at: SystemTime,
}

impl MemoryMmapFileMut {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            mmap: BytesMut::new(),
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now(),
        }
    }

    pub fn with_capacity<P: AsRef<Path>>(path: P, cap: usize) -> Self {
        Self {
            mmap: BytesMut::with_capacity(cap),
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now(),
        }
    }

    pub fn from_vec<P: AsRef<Path>>(path: P, src: Vec<u8>) -> Self {
        Self {
            mmap: BytesMut::from_iter(src),
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now(),
        }
    }

    pub fn from_string<P: AsRef<Path>>(path: P, src: String) -> Self {
        Self {
            mmap: BytesMut::from(src.as_bytes()),
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now()
        }
    }

    pub fn from_str<P: AsRef<Path>>(path: P, src: &'static str) -> Self {
        Self {
            mmap: BytesMut::from(src),
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now()
        }
    }

    pub fn from_slice<P: AsRef<Path>>(path: P, src: &[u8]) -> Self {
        Self {
            mmap: BytesMut::from(src),
            path: path.as_ref().to_path_buf(),
            create_at: SystemTime::now()
        }
    }

    pub fn into_bytes_mut(self) -> BytesMut {
        self.mmap
    }

    pub fn into_bytes(self) -> Bytes {
        self.mmap.freeze()
    }
}

impl_mmap_file_ext!(MemoryMmapFileMut);

impl MmapFileMutExt for MemoryMmapFileMut {
    #[inline(always)]
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.mmap.as_mut()
    }

    #[inline(always)]
    fn is_cow(&self) -> bool {
        false
    }

    noop_flush!();

    #[inline(always)]
    fn truncate(&mut self, max_sz: u64) -> crate::error::Result<()> {
        self.mmap.resize(max_sz as usize, 0);
        Ok(())
    }

    #[inline(always)]
    fn remove(self) -> crate::error::Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn close_with_truncate(self, _max_sz: i64) -> crate::error::Result<()> {
        Ok(())
    }
}

impl MemoryMmapFileMut {
    /// Make the memory mmap file immutable
    #[inline(always)]
    pub fn freeze(self) -> MemoryMmapFile {
        MemoryMmapFile {
            mmap: self.mmap.freeze(),
            path: self.path,
            create_at: self.create_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom, Write};
    use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
    use super::*;

    #[test]
    fn test_memory_file() {
        let mut file = MemoryMmapFileMut::new("memory.mem");
        assert_eq!(file.as_mut_slice().len(), 0);
        file.truncate(8096).unwrap(); // 1 KB
        let mut writter = file.writer(0).unwrap();
        writter.write_all("memory file test".as_bytes()).unwrap();
        writter.seek(SeekFrom::Start(100)).unwrap();
        writter.write_i8(-8).unwrap();
        writter.write_i16::<BigEndian>(-16).unwrap();
        writter.write_i32::<BigEndian>(-32).unwrap();
        writter.write_i64::<BigEndian>(-64).unwrap();
        writter.flush().unwrap();
        writter.seek(SeekFrom::End(0)).unwrap();
        drop(writter);
        let mut reader = file.reader(0).unwrap();
        let mut buf = [0; "memory file test".len()];
        reader.read_exact(&mut buf).unwrap();
        assert!(buf.eq("memory file test".as_bytes()));
        reader.seek(SeekFrom::Start(100)).unwrap();
        assert_eq!(-8, reader.read_i8().unwrap());
        assert_eq!(-16, reader.read_i16::<BigEndian>().unwrap());
        assert_eq!(-32, reader.read_i32::<BigEndian>().unwrap());
        assert_eq!(-64, reader.read_i64::<BigEndian>().unwrap());


        let mut range_writer = file.range_writer(8000, 96).unwrap();
        range_writer.write_u8(8).unwrap();
        range_writer.write_u16::<BigEndian>(16).unwrap();
        range_writer.write_u32::<BigEndian>(32).unwrap();
        range_writer.write_u64::<BigEndian>(64).unwrap();
        range_writer.flush().unwrap();

        let mut range_reader = file.range_reader(8000, 96).unwrap();
        assert_eq!(8, range_reader.read_u8().unwrap());
        assert_eq!(16, range_reader.read_u16::<BigEndian>().unwrap());
        assert_eq!(32, range_reader.read_u32::<BigEndian>().unwrap());
        assert_eq!(64, range_reader.read_u64::<BigEndian>().unwrap());

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
    }
}