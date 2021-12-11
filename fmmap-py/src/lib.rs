use fmmap::raw::DiskMmapFileMut;
use fmmap::MmapFileMut;

#[test]
fn test() {
    let m: MmapFileMut = DiskMmapFileMut::create("asd/vva.txt").unwrap().into();
}
