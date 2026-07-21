use std::fmt;

#[derive(Debug)]
pub enum Error {
    UnsupportedPlatform,
    LoadLibrary(libloading::Error),
    MissingExport {
        name: &'static str,
        source: libloading::Error,
    },
    ApiCallFailed {
        name: &'static str,
        code: u32,
    },
    MmfTooLarge(usize),
    InvalidBufferLengths {
        left: usize,
        right: usize,
    },
    FrameCountTooLarge(usize),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => f.write_str(
                "the vendor DLL is a 32-bit Windows library; use an i686-pc-windows target",
            ),
            Self::LoadLibrary(error) => write!(f, "failed to load DLL: {error}"),
            Self::MissingExport { name, source } => {
                write!(f, "DLL export {name} is unavailable: {source}")
            }
            Self::ApiCallFailed { name, code } => {
                write!(f, "{name} failed with code 0x{code:08x}")
            }
            Self::MmfTooLarge(len) => write!(f, "MMF length {len} exceeds the 32-bit API limit"),
            Self::InvalidBufferLengths { left, right } => write!(
                f,
                "left and right output buffers differ in length ({left} != {right})"
            ),
            Self::FrameCountTooLarge(frames) => {
                write!(
                    f,
                    "frame count {frames} cannot be represented by the DLL ABI"
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::LoadLibrary(error) => Some(error),
            Self::MissingExport { source, .. } => Some(source),
            _ => None,
        }
    }
}
