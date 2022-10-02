use parse_display::Display;
use std::io;

/// alias for [`Result<T, Error>`]
///
/// [`Result<T, Error>`]: struct.Error.html
pub type Result<T> = std::result::Result<T, Error>;

/// ErrorKind in this crate.
#[derive(Copy, Clone, Eq, PartialEq, Display, Debug)]
pub enum ErrorKind {
    /// unexpected EOF
    #[display("unexpected EOF")]
    EOF,

    /// IO error
    #[display("IO error")]
    IO,

    /// Truncation failed
    #[display("truncation failed")]
    TruncationFailed,

    /// unable to open file
    #[display("unable to open file")]
    OpenFailed,

    /// unable to open dir
    #[display("unable to open dir")]
    OpenDirFailed,

    /// flush file failed
    #[display("flush file failed")]
    FlushFailed,

    /// sync dir failed
    #[display("sync file failed")]
    SyncFileFailed,

    /// sync dir failed
    #[display("sync dir failed")]
    SyncDirFailed,

    /// mmap failed
    #[display("mmap failed")]
    MmapFailed,

    /// remmap failed
    #[display("remmap failed")]
    RemmapFailed,

    /// invalid range
    #[display("range start must not be greater than end: {0} <= {1}")]
    InvalidBound(usize, usize),

    /// out of range
    #[display("range end out of bounds: {0} <= {1}")]
    OutOfBound(usize, usize),

    /// call on an empty mmap file
    #[display("call on an empty mmap file")]
    InvokeEmptyMmap,

    /// not a directory
    #[cfg(not(feature = "nightly"))]
    #[display("not a directory")]
    NotADirectory,
}

enum Repr {
    Simple(ErrorKind),
    Message { kd: ErrorKind, msg: String },
    Source(Box<Source>),
    SourceMessage { msg: String, src: Box<Source> },
}

struct Source {
    kind: ErrorKind,
    error: Box<dyn std::error::Error + Send + Sync>,
}

/// Error in this crate
pub struct Error {
    repr: Repr,
}

impl Error {
    pub(crate) fn new<E>(kd: ErrorKind, src: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::_new(kd, src.into())
    }

    fn _new(kind: ErrorKind, error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Error {
            repr: Repr::Source(Box::new(Source { kind, error })),
        }
    }

    pub(crate) fn new_with_message<M>(kd: ErrorKind, msg: M) -> Self
    where
        M: Into<String>,
    {
        Self {
            repr: Repr::Message {
                kd,
                msg: msg.into(),
            },
        }
    }

    pub(crate) fn new_source_msg<M, E>(kd: ErrorKind, msg: M, src: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
        M: Into<String>,
    {
        Self {
            repr: Repr::SourceMessage {
                msg: msg.into(),
                src: Box::new(Source {
                    kind: kd,
                    error: src.into(),
                }),
            },
        }
    }

    pub(crate) fn f(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.repr {
            Repr::Simple(kd) => write!(formatter, "{}", kd),
            Repr::Source(src) => write!(formatter, "{}: {}", src.error, src.kind),
            Repr::Message { kd, msg } => write!(formatter, "{}: {}", msg, kd),
            Repr::SourceMessage { msg, src } => {
                write!(formatter, "{}: {}: {}", msg, src.kind, src.error)
            }
        }
    }

    /// Return the [`ErrorKind`] of this error
    ///
    /// [`ErrorKind`]: struct.ErrorKind.html
    pub fn kind(&self) -> ErrorKind {
        match &self.repr {
            Repr::Simple(kd) => *kd,
            Repr::Message { kd, msg: _ } => *kd,
            Repr::Source(src) => src.kind,
            Repr::SourceMessage { msg: _, src } => src.kind,
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kd: ErrorKind) -> Self {
        Self {
            repr: Repr::Simple(kd),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.f(f)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.f(f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.repr {
            Repr::Simple(_) => None,
            Repr::Source(ref c) => Some(c.error.as_ref()),
            Repr::Message { .. } => None,
            Repr::SourceMessage { msg: _, ref src } => Some(src.error.as_ref()),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::UnexpectedEof => Error::new(ErrorKind::EOF, err),
            _ => Error::new(ErrorKind::IO, err),
        }
    }
}
