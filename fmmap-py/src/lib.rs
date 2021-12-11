use fmmap::raw::DiskMmapFileMut;
use fmmap::MmapFileMut;

#[test]
fn test() {
    let _m: MmapFileMut = DiskMmapFileMut::create("asd/vva.txt").unwrap().into();
}
