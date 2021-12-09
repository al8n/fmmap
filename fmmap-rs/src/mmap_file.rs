macro_rules! read_impl {
    ($this:ident, $offset: tt, $typ:tt::$conv:tt) => {{
        const SIZE: usize = mem::size_of::<$typ>();
        // try to convert directly from the bytes
        // this Option<ret> trick is to avoid keeping a borrow on self
        // when advance() is called (mut borrow) and to call bytes() only once
        let mut buf = [0; SIZE];
        $this.read_exact(&mut buf, $offset).map(|src| unsafe { $typ::$conv(*(&src as *const _ as *const [_; SIZE])) })
    }};
}

cfg_sync!(
    mod sync_impl;
    pub use sync_impl::{MmapFileExt, MmapFileMutExt};
);

cfg_tokio!(
    mod tokio_impl;
    pub use tokio_impl::{AsyncMmapFileExt, AsyncMmapFileMutExt};
);





