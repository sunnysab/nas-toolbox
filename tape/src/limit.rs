use super::TapeDevice;
use anyhow::Result;

#[repr(C)]
#[derive(Debug)]
pub struct BlockLimit {
    /// The actual granularity is 2 raised to the power of the value.
    ///
    /// In computer science and storage technology, "granularity" refers to the smallest unit of data storage and
    /// retrieval in a magnetic tape. It determines the level of precision for accessing and transferring data on the tape.
    ///
    /// A magnetic tape is typically divided into blocks or records, with each block or record containing a certain amount
    /// of data. The gaps between these blocks or records are known as gaps. Granularity refers to the size or length of
    /// each block or record, i.e., the amount of data contained within each block or record.
    ///
    /// Smaller granularity means smaller-sized blocks or records, while larger granularity means larger-sized blocks or
    /// records. The choice of granularity affects the storage capacity of the tape, data transfer speed, and the efficiency
    /// of data access. (by ChatGPT)
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
