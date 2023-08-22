use std::os::fd::RawFd;
use anyhow::Result;

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
    /// Enable controller cache
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


#[repr(C)]
#[derive(Debug, Default)]
pub struct MtStatus {
    /// type of magnetic tape device
    _type: i16,
    /// "drive status" register (device dependent)
    dsreg: i16,
    /// "error" register (device dependent)
    erreg: i16,
    /// residual count
    resid: i16,
    /// presently operating block size
    blksiz: i32,
    /// presently operating density
    density: i32,
    /// presently operating compression
    comp: u32,
    /// blocksize for mode 0
    blksiz0: i32,
    /// blocksize for mode 1
    blksiz1: i32,
    /// blocksize for mode 2
    blksiz2: i32,
    /// blocksize for mode 3
    blksiz3: i32,
    /// density for mode 0
    density0: i32,
    /// density for mode 1
    density1: i32,
    /// density for mode 2
    density2: i32,
    /// density for mode 3
    density3: i32,
    /// compression type for mode 0 (not implemented)
    comp0: u32,
    /// compression type for mode 1 (not implemented)
    comp1: u32,
    /// compression type for mode 2 (not implemented)
    comp2: u32,
    /// compression type for mode 3 (not implemented)
    comp3: u32,
    /// relative file number of current position
    fileno: i32,
    /// relative block number of current position
    blkno: i32,
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

    nix::ioctl_read!(get_status, b'm', 2u8, MtStatus);

    pub fn status(&self) -> Result<MtStatus> {
        assert_eq!(std::mem::size_of::<MtStatus>(), 76);

        let mut result = MtStatus::default();
        unsafe {
            Self::get_status(self.fd, &mut result)?;
        }

        Ok(result)
    }
}


fn main() -> Result<()> {
    let device = TapeDevice::open("/dev/sa0")?;
    let status = device.status()?;

    println!("{:#?}", status);
    Ok(())
}
