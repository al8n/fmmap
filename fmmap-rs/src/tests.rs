use rand::{thread_rng, Rng};
use std::path::PathBuf;

#[cfg(not(windows))]
pub fn get_random_filename() -> PathBuf {
    let mut rng = thread_rng();
    let mut filename = std::env::temp_dir();
    filename.push(rng.gen::<u32>().to_string());
    filename.set_extension("txt");
    filename
}

#[cfg(windows)]
pub fn get_random_filename() -> PathBuf {
    let mut rng = thread_rng();
    let mut filename = std::env::temp_dir();
    filename.push("fmmap");
    let _ = std::fs::create_dir(&filename);
    filename.push(rng.gen::<u32>().to_string());
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
    use crate::raw::DiskMmapFileMut;
    use crate::raw::MemoryMmapFileMut;
    use crate::{MmapFileExt, MmapFileMut, MmapFileMutExt, MmapFileReaderExt, MmapFileWriterExt};

    sync_tests!(
        [test_memory_file_mut, {
            MemoryMmapFileMut::new("memory.txt")
        }],
        [test_mmap_file_mut, {
            let mut file =
                MmapFileMut::from(DiskMmapFileMut::create(get_random_filename()).unwrap());
            file.set_remove_on_drop(true);
            assert!(file.get_remove_on_drop());
            file
        }],
    );
}

#[cfg(feature = "tokio-async")]
mod axync {
    macro_rules! tokio_async_tests {
        ($([$test_fn: ident, $init: block]), +$(,)?) => {
            use std::io::SeekFrom;
            use scopeguard::defer;
            use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

            const SANITY_TEXT: &'static str = "Hello, async file!";
            const MODIFIED_SANITY_TEXT: &'static str = "Hello, modified async file!";

            $(
                #[cfg(feature = "tokio-async")]
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
    use crate::raw::tokio::AsyncDiskMmapFileMut;
    use crate::raw::tokio::AsyncMemoryMmapFileMut;
    use crate::tokio::{AsyncMmapFileExt, AsyncMmapFileMut, AsyncMmapFileMutExt};

    tokio_async_tests!(
        [test_async_memory_file_mut, {
            AsyncMemoryMmapFileMut::new("memory.txt")
        }],
        [test_async_mmap_file_mut, {
            let mut file = AsyncMmapFileMut::from(
                AsyncDiskMmapFileMut::create(get_random_filename())
                    .await
                    .unwrap(),
            );
            file.set_remove_on_drop(true);
            assert!(file.get_remove_on_drop());
            file
        }],
    );
}
