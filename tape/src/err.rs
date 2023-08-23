use super::TapeDevice;
use anyhow::Result;

/// structure for MTIOCERRSTAT - tape get error status command
/// really only supported for SCSI tapes right now
#[derive(Debug, Copy, Clone)]
pub struct ScsiTapeErrors {
    // These are latched from the last command that had a SCSI
    // Check Condition noted for these operations. The act
    // of issuing an MTIOCERRSTAT unlatches and clears them.
    /// Last Sense Data For Data I/O
    io_sense: [u8; 32],
    /// residual count from last Data I/O
    io_resid: i32,
    /// Command that Caused the Last Data Sense
    io_cdb: [u8; 16],
    /// Last Sense Data For Control I/O
    ctl_sense: [u8; 32],
    /// residual count from last Control I/O
    ctl_resid: i32,
    /// Command that Caused the Last Control Sense
    ctl_cdb: [u8; 16],

    // These are the read and write cumulative error counters.
    // (how to reset cumulative error counters is not yet defined).
    // (not implemented as yet but space is being reserved for them)
    _wterr: ErrorCounter,
    _rderr: ErrorCounter,
}

#[derive(Debug, Copy, Clone)]
pub struct ErrorCounter {
    /// total # retries performed
    retries: u32,
    /// total # corrections performed
    corrected: u32,
    /// total # corrections successful
    processed: u32,
    /// total # corrections/retries failed
    failures: u32,
    /// total # bytes processed
    nbytes: u64,
}

#[repr(C)]
pub union MtErrStat {
    scsi_err_stat: ScsiTapeErrors,
    _reserved_padding: [u8; 256],
}

mod ioctl_func {
    use super::MtErrStat;

    nix::ioctl_read!(read_error_status, b'm', 7u8, MtErrStat);
}

impl TapeDevice {
    /// Output (and clear) error status information about this lib.  
    ///
    /// For every normal operation (e.g., a read or a write) and every control operation (e.g,, a rewind), the
    /// driver stores up the last command executed and it is associated status and any residual counts (if any).
    ///
    /// This function retrieves and returns this information.  If possible, this also clears any latched error information.
    /// (From FreeBSD manual)
    pub fn get_last_error(&self) -> Result<ScsiTapeErrors> {
        let result = unsafe {
            let mut err_stat: MtErrStat = std::mem::zeroed();
            ioctl_func::read_error_status(self.fd, &mut err_stat)?;

            err_stat.scsi_err_stat
        };

        Ok(result)
    }
}
