use super::TapeDevice;
use anyhow::Result;

#[derive(Debug)]
pub enum Operation {
    /// Write an end-of-file record
    WriteEof = 0,
    /// Forward space file
    ForwardSpaceFile = 1,
    /// Backward space file
    BackwardSpaceFile = 2,
    /// Forward space record
    ForwardSpaceRecord = 3,
    /// Backward space record
    BackwardSpaceRecord = 4,
    /// Rewind
    Rewind = 5,
    /// Rewind and put the drive offline
    Offline = 6,
    /// No operation, sets status only
    NOP = 7,
    /// Enable controller cache
    EnableCache = 8,
    /// Disable controller cache
    DisableCache = 9,
    /// Set block size for lib
    SetBlockSize = 10,
    /// Set density values for lib
    SetDensity = 11,
    /// Erase to EOM
    EraseToEnd = 12,
    /// Space to EOM
    JumpToEnd = 13,
    /// Select compression mode 0=off, 1=def
    SetCompression = 14,
    /// Re-tension tape
    Retension = 15,
    /// Write setmark(s)
    WriteSetmark = 16,
    /// Forward space setmark
    ForwardSpaceSetmark = 17,
    /// Backward space setmark
    BackwardSpaceSetmark = 18,
    /// Load tape in drive
    Load = 19,
    /// Write an end-of-file record without waiting
    WriteEofImmediately = 20,
}

#[repr(C)]
pub struct MtOp {
    /// Operations defined above
    op: u16,
    /// How many of them.
    /// If you don't understand, see `man mt`
    count: i32,
}

mod ioctl_func {
    use super::MtOp;

    nix::ioctl_write_ptr!(tape_op, b'm', 1u8, MtOp);
}

impl TapeDevice {
    fn do_tape_op(&self, op: Operation, count: u32) -> Result<i32> {
        let ret = unsafe {
            let mut mt_op: MtOp = std::mem::zeroed();
            mt_op.op = op as u16;
            mt_op.count = count as i32;
            ioctl_func::tape_op(self.fd, &mt_op)?
        };

        Ok(ret)
    }

    pub fn write_eof(&self, count: u32) -> Result<()> {
        self.do_tape_op(Operation::WriteEof, count).map(|_| ())
    }

    pub fn write_eof_immediately(&self, count: u32) -> Result<()> {
        self.do_tape_op(Operation::WriteEofImmediately, count).map(|_| ())
    }

    /// DDS drive only
    pub fn write_setmark(&self, count: u32) -> Result<()> {
        self.do_tape_op(Operation::WriteSetmark, count).map(|_| ())
    }

    pub fn forward_space_file(&self, count: u32) -> Result<()> {
        self.do_tape_op(Operation::ForwardSpaceFile, count).map(|_| ())
    }

    pub fn backward_space_file(&self, count: u32) -> Result<()> {
        self.do_tape_op(Operation::BackwardSpaceFile, count).map(|_| ())
    }

    pub fn forward_space_record(&self, count: u32) -> Result<()> {
        self.do_tape_op(Operation::ForwardSpaceRecord, count).map(|_| ())
    }

    pub fn backward_space_record(&self, count: u32) -> Result<()> {
        self.do_tape_op(Operation::BackwardSpaceRecord, count).map(|_| ())
    }

    /// DDS drive only
    pub fn forward_space_setmark(&self, count: u32) -> Result<()> {
        self.do_tape_op(Operation::ForwardSpaceSetmark, count).map(|_| ())
    }

    /// DDS drive only
    pub fn backward_space_setmark(&self, count: u32) -> Result<()> {
        self.do_tape_op(Operation::BackwardSpaceSetmark, count).map(|_| ())
    }

    pub fn rewind(&self) -> Result<()> {
        self.do_tape_op(Operation::Rewind, 0).map(|_| ())
    }

    pub fn rewind_and_offline(&self) -> Result<()> {
        self.do_tape_op(Operation::Offline, 0).map(|_| ())
    }

    pub fn load(&self) -> Result<()> {
        self.do_tape_op(Operation::Load, 0).map(|_| ())
    }

    pub fn set_block_size(&self, size: u32) -> Result<()> {
        self.do_tape_op(Operation::SetBlockSize, size).map(|_| ())
    }

    pub fn set_density(&self, code: u32) -> Result<()> {
        self.do_tape_op(Operation::SetDensity, code).map(|_| ())
    }

    pub fn set_compression(&self, enable: bool) -> Result<()> {
        self.do_tape_op(Operation::SetCompression, enable as u32).map(|_| ())
    }

    /// Zero represents doing quickly
    pub fn erase(&self, count: u32) -> Result<()> {
        self.do_tape_op(Operation::EraseToEnd, count).map(|_| ())
    }

    pub fn jump_to_eom(&self) -> Result<()> {
        self.do_tape_op(Operation::JumpToEnd, 0).map(|_| ())
    }

    pub fn retension(&self) -> Result<()> {
        self.do_tape_op(Operation::Retension, 0).map(|_| ())
    }
}
