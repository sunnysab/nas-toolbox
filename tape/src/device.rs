mod limit;
mod locate;
mod status;
mod status_ex;

use anyhow::Result;
pub use limit::BlockLimit;
pub use locate::{Location, LocationBuilder};
pub use status::{Density, DriverState, TapeStatus};
use std::os::fd::RawFd;

enum MtOperation {
    /// Write an end-of-file record
    MtWEOF = 0,
    /// Forward space file
    MtFSF = 1,
    /// Backward space file
    MtBSF = 2,
    /// Forward space record
    MtFSR = 3,
    /// Backward space record
    MtBSR = 4,
    /// Rewind
    MtREW = 5,
    /// Rewind and put the drive offline
    MtOFFL = 6,
    /// No operation, sets status only
    MtNOP = 7,
    /// Enable controller cach
    MtCACHE = 8,
    /// Disable controller cache
    MtNOCACHE = 9,
    /// Set block size for device
    MtSETBSIZ = 10,
    /// Set density values for device
    MtSETDNSTY = 11,
    /// Erase to EOM
    MtERASE = 12,
    /// Space to EOM
    MtEOD = 13,
    /// Select compression mode 0=off, 1=def
    MtCOMP = 14,
    /// Re-tension tape
    MtRETENS = 15,
    /// Write setmark(s)
    MtWSS = 16,
    /// Forward space setmark
    MtFSS = 17,
    /// Backward space setmark
    MtBSS = 18,
    /// Load tape in drive
    MtLOAD = 19,
    /// Write an end-of-file record without waiting
    MtWEOFI = 20,
}

pub struct TapeDevice {
    fd: RawFd,
}

impl TapeDevice {
    pub fn open<P: nix::NixPath + ?Sized>(path: &P) -> Result<Self> {
        use nix::fcntl::OFlag;
        use nix::sys::stat::Mode;

        let fd = nix::fcntl::open(path, OFlag::O_RDWR, Mode::all())?;
        Ok(Self { fd })
    }
}
