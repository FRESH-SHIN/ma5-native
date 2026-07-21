//! Runtime bindings for a user-supplied MA-5 software emulator DLL.
//!
//! This crate does not contain, download, or redistribute Yamaha software.

mod error;

pub use error::{Error, Result};

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod smw5;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub use smw5::{MaSound, MmfPlayback, RenderedPcm};

#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
mod unsupported {
    use std::path::Path;

    use crate::{Error, Result};

    /// Placeholder on platforms that cannot load the original 32-bit DLLs.
    pub struct MaSound;

    impl MaSound {
        pub fn load(_path: impl AsRef<Path>) -> Result<Self> {
            Err(Error::UnsupportedPlatform)
        }
    }
}

#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
pub use unsupported::MaSound;
