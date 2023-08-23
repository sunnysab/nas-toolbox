use super::TapeDevice;
use anyhow::Result;
use std::result;

#[repr(C)]
#[derive(Debug)]
pub struct BlockLimit {
    granularity: u32,
    min_block_length: u32,
    max_block_length: u32,
}

mod ioctl_func {
    use super::BlockLimit;

    nix::ioctl_read!(read_block_limit, b'm', 9u8, BlockLimit);
}

impl TapeDevice {
    pub fn read_block_limit(&self) -> Result<BlockLimit> {
        let result = unsafe {
            let mut limit: BlockLimit = std::mem::zeroed();

            ioctl_func::read_block_limit(self.fd, &mut limit)?;
            limit
        };

        Ok(result)
    }
}
