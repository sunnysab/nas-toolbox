use crate::TapeDevice;
use anyhow::{bail, Context, Result};
use strum::{EnumIter, EnumString, FromRepr};

#[derive(Debug)]
pub struct Density {
    pub code: u32,
    /// Bits per mm
    pub bpmm: u32,
    /// Bits per inch
    pub bpi: u32,
    /// Description
    pub description: &'static str,
}

/// Copied from `freebsd-src/lib/libmt/mtlib.c`,
/// which are originally from T10 Project 997D
static DENSITIES: [Density; 14] = [
    Density {
        code: 0x40,
        bpmm: 4880,
        bpi: 123952,
        description: "LTO-1",
    },
    Density {
        code: 0x42,
        bpmm: 7398,
        bpi: 187909,
        description: "LTO-2",
    },
    Density {
        code: 0x44,
        bpmm: 9638,
        bpi: 244805,
        description: "LTO-3",
    },
    Density {
        code: 0x46,
        bpmm: 12725,
        bpi: 323215,
        description: "LTO-4",
    },
    Density {
        code: 0x40,
        bpmm: 4880,
        bpi: 123952,
        description: "LTO-1",
    },
    Density {
        code: 0x42,
        bpmm: 7398,
        bpi: 187909,
        description: "LTO-2",
    },
    Density {
        code: 0x44,
        bpmm: 9638,
        bpi: 244805,
        description: "LTO-3",
    },
    Density {
        code: 0x46,
        bpmm: 12725,
        bpi: 323215,
        description: "LTO-4",
    },
    Density {
        code: 0x58,
        bpmm: 15142,
        bpi: 384607,
        description: "LTO-5",
    },
    Density {
        code: 0x5A,
        bpmm: 15142,
        bpi: 384607,
        description: "LTO-6",
    },
    Density {
        code: 0x5C,
        bpmm: 19107,
        bpi: 485318,
        description: "LTO-7",
    },
    Density {
        code: 0x5D,
        bpmm: 19107,
        bpi: 485318,
        description: "LTO-M8",
    },
    Density {
        code: 0x5E,
        bpmm: 20669,
        bpi: 524993,
        description: "LTO-8",
    },
    Density {
        code: 0x60,
        bpmm: 23031,
        bpi: 584987,
        description: "LTO-9",
    },
];

static UNKNOWN_DENSITY: Density = Density {
    code: 0,
    bpmm: 0,
    bpi: 0,
    description: "Unknown",
};

impl Density {
    fn get(code: u32) -> &'static Self {
        for predefined in &DENSITIES {
            if predefined.code == code {
                return predefined;
            }
        }
        &UNKNOWN_DENSITY
    }
}

#[derive(Debug)]
pub enum BlockSize {
    Variable,
    Fixed(u32),
}

impl From<i32> for BlockSize {
    fn from(value: i32) -> Self {
        if value == 0 {
            Self::Variable
        } else {
            Self::Fixed(value as u32)
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct RawStatus {
    /// type of magnetic tape driver
    pub _type: i16,
    /// "drive status" register (lib dependent)
    pub dsreg: i16,
    /// "error" register (lib dependent)
    pub erreg: i16,
    /// residual count
    pub resid: i16,
    /// presently operating block size
    pub blksiz: i32,
    /// presently operating density
    pub density: i32,
    /// presently operating compression
    pub comp: u32,
    /// blocksize for mode 0
    pub blksiz0: i32,
    /// blocksize for mode 1
    pub blksiz1: i32,
    /// blocksize for mode 2
    pub blksiz2: i32,
    /// blocksize for mode 3
    pub blksiz3: i32,
    /// density for mode 0
    pub density0: i32,
    /// density for mode 1
    pub density1: i32,
    /// density for mode 2
    pub density2: i32,
    /// density for mode 3
    pub density3: i32,
    /// compression type for mode 0 (not implemented)
    pub comp0: u32,
    /// compression type for mode 1 (not implemented)
    pub comp1: u32,
    /// compression type for mode 2 (not implemented)
    pub comp2: u32,
    /// compression type for mode 3 (not implemented)
    pub comp3: u32,
    /// relative file number of current position
    pub fileno: i32,
    /// relative block number of current position
    pub blkno: i32,
}

#[derive(Debug, EnumString, FromRepr)]
pub enum DriverState {
    /// Unknown
    #[strum(serialize = "Unknown")]
    Nil = 0,
    /// Doing Nothing
    #[strum(serialize = "Doing Nothing")]
    Rest = 1,
    /// Communicating with tape (but no motion)
    #[strum(serialize = "Communicating with tape (but no motion)")]
    Busy = 2,
    /// Writing
    #[strum(serialize = "Writing")]
    Writing = 20,
    /// Writing Filemarks
    #[strum(serialize = "Writing Filemarks")]
    WritingFilemarks = 21,
    /// Erasing
    #[strum(serialize = "Erasing")]
    Erasing = 22,
    /// Reading
    #[strum(serialize = "Reading")]
    Reading = 30,
    /// Spacing Forward
    #[strum(serialize = "Spacing Forward")]
    SpacingForward = 40,
    /// Spacing Reverse
    #[strum(serialize = "Spacing Reverse")]
    SpacingReverse = 41,
    /// Hardware Positioning (direction unknown)
    #[strum(serialize = "Hardware Positioning (direction unknown)")]
    Pos = 42,
    /// Rewinding
    #[strum(serialize = "Rewinding")]
    Rewinding = 43,
    /// Retensioning
    #[strum(serialize = "Retensioning")]
    Retensioning = 44,
    /// Unloading
    #[strum(serialize = "Unloading")]
    Unloading = 45,
    /// Loading
    #[strum(serialize = "Loading")]
    Loading = 46,
}

#[derive(EnumString, EnumIter, Clone, Copy, Debug)]
pub enum Compression {
    #[strum(serialize = "Off")]
    Off,
    #[strum(serialize = "On")]
    On,
    #[strum(serialize = "IDRC Algorithm")]
    Idrc,
    #[strum(serialize = "DCLZ Algorithm")]
    Dclz,

    Unknown,
}

impl From<u32> for Compression {
    fn from(value: u32) -> Self {
        match value {
            0 => Compression::Off,
            1 | 0xffffffff => Compression::On,
            0x10 => Compression::Idrc,
            0x20 => Compression::Dclz,
            _ => Compression::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct TapeStatus {
    pub state: DriverState,
    pub block_size: BlockSize,
    pub density: &'static Density,
    pub compression: Compression,

    /// relative file number of current position
    pub file_no: usize,
    /// relative block number of current position
    pub block_no: usize,
    /// Residual count
    pub residual: usize,
}

impl TryFrom<RawStatus> for TapeStatus {
    type Error = anyhow::Error;

    fn try_from(raw: RawStatus) -> Result<Self> {
        let state = DriverState::from_repr(raw.dsreg as usize)
            .with_context(|| format!("Unknown tape driver state from dsreg: {}", raw.dsreg))?;

        let density = Density::get(raw.density as u32);
        let compression = Compression::from(raw.comp);

        let result = TapeStatus {
            state,
            density,
            compression,
            block_size: BlockSize::from(raw.blksiz),
            file_no: raw.fileno as usize,
            block_no: raw.blkno as usize,
            residual: raw.resid as usize,
        };
        Ok(result)
    }
}

mod ioctl_func {
    use super::RawStatus;

    nix::ioctl_read!(get_status, b'm', 2u8, RawStatus);
}

impl TapeDevice {
    pub fn status(&self) -> Result<TapeStatus> {
        assert_eq!(std::mem::size_of::<RawStatus>(), 76);

        let mut raw_status = RawStatus::default();
        unsafe {
            ioctl_func::get_status(self.fd, &mut raw_status)?;
        }

        /* #define MT_ISAR  0x07, scsi lib */
        if raw_status._type != 0x07 {
            bail!("Your tape lib is not of SCSI.");
        }
        TapeStatus::try_from(raw_status)
    }
}
