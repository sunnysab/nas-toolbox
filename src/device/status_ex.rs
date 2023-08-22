use std::ffi::CStr;
use anyhow::{bail, Result};
use serde::Deserialize;
use crate::TapeDevice;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct MtextgetNode {
    /// Device driver name, such as `sa(8)`.
    periph_name: String,
    /// Device id. For device `/dev/sa0`, this value could be `0`.
    unit_number: u32,
    /// SCSI Vendor ID
    vendor: String,
    /// SCSI Product ID
    product: String,
    /// SCSI Revision
    revision: String,
    /// Serial Number
    serial_num: String,
    /// Maximum I/O size allowed by driver and controller
    maxio: u32,
    /// Maximum I/O size reported by controller
    cpi_maxio: u32,
    /// Maximum block size supported by tape drive and media
    max_blk: u32,
    /// Minimum block size supported by tape drive and media
    min_blk: u32,
    /// Block granularity supported by tape drive and media
    blk_gran: u32,
    /// Maximum possible I/O size
    max_effective_iosize: u32,
    /// Set to 1 for fixed block mode, 0 for variable block
    fixed_mode: i32,
    /// Set to 1 if compression is supported, 0 if not
    compression_supported: i32,
    /// Set to 1 if compression is enabled, 0 if not
    compression_enabled: i32,
    /// Numeric compression algorithm
    compression_algorithm: u32,
    // protection node described outside
    // protection: Protection,

    /// Block size reported by drive or set by user
    media_blocksize: u32,
    /// Calculated file number, -1 if unknown
    calculated_fileno: i64,
    /// Calculated block number relative to file, set to -1 if unknown
    calculated_rel_blkno: i64,
    /// File number reported by drive, -1 if unknown
    reported_fileno: i64,
    /// Block number relative to BOP/BOT reported by drive, -1 if unknown
    reported_blkno: i64,
    /// Current partition number, 0 is the default
    partition: i64,
    /// Set to 1 if drive is at the beginning of partition/tape, 0 if not, -1 if unknown
    bop: i32,
    /// Set to 1 if drive is past early warning, 0 if not, -1 if unknown
    eop: i32,
    /// Set to 1 if drive is past programmable early warning, 0 if not, -1 if unknown
    bpew: i32,
    /// Residual for the last I/O
    residual: i64,
    /// Current state of the driver
    dsreg: i32,
    // density node described outside
    // mtdensity: Mtdensity,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct Protection {
    /// Set to 1 if protection information is supported
    protection_supported: i32,
    /// Current Protection Method
    prot_method: u32,
    /// Length of Protection Information
    pi_length: u32,
    /// Check Protection on Writes
    lbp_w: u32,
    /// Check and Include Protection on Reads
    lbp_r: u32,
    /// Transfer Protection Information for RECOVER BUFFERED DATA command
    rbdp: u32,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct Mtdensity {
    /// Current Medium Density Code
    media_density: u8,
    density_report: Vec<DensityReport>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct DensityReport {
    /// Medium type report
    medium_type_report: i32,
    /// Media report
    media_report: i32,
    density_entry: Vec<DensityEntry>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct DensityEntry {
    /// Primary Density Code
    primary_density_code: u8,
    /// Secondary Density Code
    secondary_density_code: u8,
    /// Density Flags
    density_flags: u32,
    /// Bits per mm
    bits_per_mm: u32,
    /// Media width
    media_width: u32,
    /// Number of Tracks
    tracks: u32,
    /// Capacity (in bytes)
    capacity: u32,
    /// Assigning Organization
    assigning_org: String,
    /// Density Name
    density_name: String,
    /// Description
    description: String,

    /* additional fields for medium type report */

    /// Medium type report
    medium_type: Option<u8>,
    /// Number of Density Codes
    num_density_codes: Option<i8>,

    density_code_list: Option<DensityCodeList>,
    /// Medium length
    medium_length: Option<u32>,
    /// Medium type name
    medium_type_name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct DensityCodeList {
    /// Density Code
    density_code: Vec<u8>,
}


#[repr(C)]
#[derive(Debug)]
enum StatusExtResult {
    None,
    Ok,
    NeedMoreSpace,
    GetError,
}

#[repr(C)]
#[derive(Debug)]
pub struct RawStatusEx {
    alloc_len: u32,
    xml: *const u8,
    fill_len: u32,
    result: StatusExtResult,
    err_str: [u8; 128],
    reserved: [u8; 64],
}


mod ioctl_func {
    use super::RawStatusEx;

    nix::ioctl_readwrite!(get_status_ex, b'm', 11u8, RawStatusEx);
}


impl TapeDevice {

    unsafe fn status_ex_get_xml(&self) -> Result<Option<String>> {
        assert_eq!(std::mem::size_of::<RawStatusEx>(), 216);

        const ALLOC_LEN: usize = 32768;

        let mut buffer = [0u8; ALLOC_LEN];

        let mut raw_status: RawStatusEx = std::mem::zeroed();
        raw_status.alloc_len = ALLOC_LEN as u32;
        raw_status.xml = buffer.as_mut_ptr();
        ioctl_func::get_status_ex(self.fd, &mut raw_status)?;

        match raw_status.result {
            StatusExtResult::None => Ok(None),
            StatusExtResult::Ok => {
                let len = raw_status.fill_len as usize;
                let xml_content = String::from_raw_parts(buffer.as_mut_ptr(), len, len);
                Ok(Some(xml_content))
            }
            StatusExtResult::NeedMoreSpace => {
                bail!("Buffer is too small, adjust ALLOC_LEN up and try again.")
            }
            StatusExtResult::GetError => {
                let message = CStr::from_ptr(raw_status.err_str.as_mut_ptr() as *mut libc::c_char).to_str().unwrap();
                bail!("{message}")
            }
        }
    }
    pub fn status_ex(&self) -> Result<Option<()>> {
        let xml = match unsafe { self.status_ex_get_xml()? } {
            Some(content) => content,
            None => return Ok(None),
        };

        Ok(Some(()))
    }
}