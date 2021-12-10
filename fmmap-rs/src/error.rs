use std::io;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unexpected EOF")]
    EOF,

    #[error("IO error: {0}")]
    IO(#[from] io::Error),

    #[error("truncation failed: {0}")]
    TruncationFailed(String),

    #[error("unable to open file: {0}")]
    OpenFailed(String),

    #[error("unable to open dir: {0}")]
    OpenDirFailed(String),

    #[error("flush file failed: {0}")]
    FlushFailed(String),

    #[error("sync dir failed: {0}")]
    SyncDirFailed(String),

    #[error("mmap failed: {0}")]
    MmapFailed(String),

    #[error("remmap failed: {0}")]
    RemmapFailed(String),
}