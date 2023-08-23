mod eot;
mod err;
mod limit;
mod locate;
mod operate;
mod status;
mod status_ex;

use anyhow::Result;
use std::os::fd::RawFd;

pub use eot::EotModel;
pub use err::{ErrorCounter, ScsiTapeErrors};
pub use limit::BlockLimit;
pub use locate::{Location, LocationBuilder};
pub use operate::Operation;
pub use status::{Density, DriverState, TapeStatus};

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
