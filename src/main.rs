mod device;

use std::os::fd::RawFd;
use anyhow::Result;

pub struct DriverLimit {

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


fn main() -> Result<()> {
    let device = TapeDevice::open("/dev/sa0")?;
    let status = device.status()?;

    println!("{:#?}", status);
    Ok(())
}
