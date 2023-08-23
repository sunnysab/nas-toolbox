use super::TapeDevice;
use anyhow::Result;

enum MtLocateDestType {
    MtLocateDestObject = 0x00,
    MtLocateDestFile = 0x01,
    MtLocateDestSet = 0x02,
    MtLocateDestEod = 0x03,
}

enum MtLocateBam {
    MtLocateBamImplicit = 0x00,
    MtLocateBamExplicit = 0x01,
}

enum MtLocateFlags {
    MtLocateFlagImmed = 0x01,
    MtLocateFlagChangePart = 0x02,
}

#[repr(C)]
pub struct MtLocate {
    flags: u32,
    dest_type: u32,
    block_address_mode: u32,
    partition: i64,
    logical_id: u64,
    reserved: [u8; 64],
}

enum Target {
    File(u64),
    Block(u64),
    Setmark(u64),
    Eod,
}

pub struct LocationBuilder {
    immediate: bool,
    to_partition: Option<i64>,
}

impl Default for LocationBuilder {
    fn default() -> Self {
        Self {
            immediate: false,
            to_partition: None,
        }
    }
}

impl LocationBuilder {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn immediate(mut self, val: bool) -> Self {
        self.immediate = val;
        self
    }

    pub fn change_partition(mut self, partition: i64) -> Self {
        self.to_partition = Some(partition);
        self
    }

    pub fn file(self, file: u64) -> Location {
        Location {
            target: Target::File(file),
            immediate: self.immediate,
            to_partition: self.to_partition,
        }
    }

    pub fn block(self, block: u64) -> Location {
        Location {
            target: Target::Block(block),
            immediate: self.immediate,
            to_partition: self.to_partition,
        }
    }

    pub fn setmark(self, setmark: u64) -> Location {
        Location {
            target: Target::Setmark(setmark),
            immediate: self.immediate,
            to_partition: self.to_partition,
        }
    }

    pub fn end_of_data(self) -> Location {
        Location {
            target: Target::Eod,
            immediate: self.immediate,
            to_partition: self.to_partition,
        }
    }
}

pub struct Location {
    target: Target,
    immediate: bool,
    to_partition: Option<i64>,
}

mod ioctl_func {
    use super::MtLocate;

    nix::ioctl_write_ptr!(locate, b'm', 10u8, MtLocate);
    nix::ioctl_read!(rdspos, b'm', 5u8, u32);
    nix::ioctl_write_ptr!(slocate, b'm', 5u8, u32);
}

impl TapeDevice {
    pub fn locate_to(&self, location: &Location) -> Result<u32> {
        assert_eq!(std::mem::size_of::<MtLocate>(), 96);

        let mut param: MtLocate = unsafe { std::mem::zeroed() };
        if location.immediate {
            param.flags |= MtLocateFlags::MtLocateFlagImmed as u32;
        }
        if let Some(partition) = location.to_partition {
            param.partition = partition;
            param.flags |= MtLocateFlags::MtLocateFlagChangePart as u32;
        }
        param.block_address_mode = MtLocateBam::MtLocateBamImplicit as u32;

        match location.target {
            Target::File(file) => {
                param.dest_type = MtLocateDestType::MtLocateDestFile as u32;
                param.logical_id = file;
            }
            Target::Block(block) => {
                param.dest_type = MtLocateDestType::MtLocateDestObject as u32;
                param.logical_id = block;
            }
            Target::Setmark(setmark) => {
                param.dest_type = MtLocateDestType::MtLocateDestSet as u32;
                param.logical_id = setmark;
            }
            Target::Eod => {
                param.dest_type = MtLocateDestType::MtLocateDestEod as u32;
            }
        }
        // Note: `/dev/nsa0` is needed, while operation on `/dev/sa0` leads always leads to status BOP.
        let ret = unsafe { ioctl_func::locate(self.fd, &mut param)? };
        Ok(ret as u32)
    }

    pub fn read_scsi_pos(&self) -> Result<u32> {
        let mut result = 0u32;
        unsafe {
            ioctl_func::rdspos(self.fd, &mut result)?;
        }
        Ok(result)
    }

    pub fn write_scsi_pos(&self, pos: u32) -> Result<()> {
        let mut _result = pos;
        unsafe {
            ioctl_func::slocate(self.fd, &mut _result)?;
        }
        Ok(())
    }
}
