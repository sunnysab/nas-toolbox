use super::{DriverState, TapeDevice};
use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use std::ffi::CStr;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct TapeStatusEx {
    /// Device driver name, such as `sa(8)`.
    pub periph_name: String,
    /// Device id. For lib `/dev/sa0`, this value could be `0`.
    pub unit_number: u32,
    /// SCSI Vendor ID
    pub vendor: String,
    /// SCSI Product ID
    pub product: String,
    /// SCSI Revision
    pub revision: String,
    /// Serial Number
    pub serial_num: String,
    /// Maximum I/O size allowed by driver and controller
    pub maxio: u32,
    /// Maximum I/O size reported by controller
    pub cpi_maxio: u32,
    /// Maximum block size supported by tape drive and media
    pub max_blk: u32,
    /// Minimum block size supported by tape drive and media
    pub min_blk: u32,
    /// Block granularity supported by tape drive and media
    pub blk_gran: u32,
    /// Maximum possible I/O size
    pub max_effective_iosize: u32,
    /// Set to 1 for fixed block mode, 0 for variable block
    pub fixed_mode: i32,
    /// Set to 1 if compression is supported, 0 if not
    pub compression_supported: i32,
    /// Set to 1 if compression is enabled, 0 if not
    pub compression_enabled: i32,
    /// Numeric compression algorithm
    pub compression_algorithm: u32,
    /// protection node described outside
    pub protection: Protection,

    /// Block size reported by drive or set by user
    pub media_blocksize: u32,
    /// Calculated file number, -1 if unknown
    pub calculated_fileno: i64,
    /// Calculated block number relative to file, set to -1 if unknown
    pub calculated_rel_blkno: i64,
    /// File number reported by drive, -1 if unknown
    pub reported_fileno: i64,
    /// Block number relative to BOP/BOT reported by drive, -1 if unknown
    pub reported_blkno: i64,
    /// Current partition number, 0 is the default
    pub partition: i64,
    /// Set to 1 if drive is at the beginning of partition/tape, 0 if not, -1 if unknown
    pub bop: i32,
    /// Set to 1 if drive is past early warning, 0 if not, -1 if unknown
    pub eop: i32,
    /// Set to 1 if drive is past programmable early warning, 0 if not, -1 if unknown
    pub bpew: i32,
    /// Residual for the last I/O
    pub residual: i64,
    /// Current state of the driver
    pub dsreg: i32,
    /// density node described outside
    pub mtdensity: MtDensity,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Protection {
    /// Set to 1 if protection information is supported
    pub protection_supported: i32,
    /// Current Protection Method
    pub prot_method: u32,
    /// Length of Protection Information
    pub pi_length: u32,
    /// Check Protection on Writes
    pub lbp_w: u32,
    /// Check and Include Protection on Reads
    pub lbp_r: u32,
    /// Transfer Protection Information for RECOVER BUFFERED DATA command
    pub rbdp: u32,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct MtDensity {
    /// Current Medium Density Code
    pub media_density: u32,
    pub density_report: Vec<DensityReport>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct DensityReport {
    /// Medium type report
    pub medium_type_report: i32,
    /// Media report
    pub media_report: i32,
    pub density_entry: Vec<DensityEntry>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct DensityEntry {
    /// Primary Density Code
    pub primary_density_code: u8,
    /// Secondary Density Code
    pub secondary_density_code: u8,
    /// Density Flags
    pub density_flags: String,
    /// Bits per mm
    pub bits_per_mm: u32,
    /// Media width
    pub media_width: u32,
    /// Number of Tracks
    pub tracks: u32,
    /// Capacity (in bytes)
    pub capacity: u32,
    /// Assigning Organization
    pub assigning_org: String,
    /// Density Name
    pub density_name: String,
    /// Description
    pub description: String,

    /* additional fields for medium type report */
    /// Medium type report
    pub medium_type: Option<u8>,
    /// Number of Density Codes
    pub num_density_codes: Option<i8>,

    pub density_code_list: Option<DensityCodeList>,
    /// Medium length
    pub medium_length: Option<u32>,
    /// Medium type name
    pub medium_type_name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct DensityCodeList {
    /// Density Code
    pub density_code: Vec<u8>,
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
                let cstr = CStr::from_ptr(buffer.as_ptr() as *const i8);
                let xml_content = cstr.to_string_lossy().to_string();
                Ok(Some(xml_content))
            }
            StatusExtResult::NeedMoreSpace => {
                bail!("Buffer is too small, adjust ALLOC_LEN up and try again.")
            }
            StatusExtResult::GetError => {
                let message = CStr::from_ptr(raw_status.err_str.as_mut_ptr() as *mut libc::c_char)
                    .to_str()
                    .unwrap();
                bail!("{message}")
            }
        }
    }
    pub fn status_ex(&self) -> Result<Option<TapeStatusEx>> {
        let xml = match unsafe { self.status_ex_get_xml()? } {
            Some(content) => content,
            None => return Ok(None),
        };

        // TODO: We need a specified xml parser to deal with it
        // DensityEntry::density_flags should be a integer, which represents in hex in xml.
        let result: TapeStatusEx = serde_xml_rs::from_str(&xml)?;
        Ok(Some(result))
    }

    pub fn protect(&self) -> Result<Option<Protection>> {
        let status_ex = self.status_ex()?;
        let protection = status_ex.map(|status| status.protection);

        Ok(protection)
    }

    pub fn density(&self) -> Result<Option<MtDensity>> {
        let status_ex = self.status_ex()?;
        let density = status_ex.map(|status| status.mtdensity);

        Ok(density)
    }

    pub fn flag(&self) -> Result<Option<DriverState>> {
        let status_ex = match self.status_ex()? {
            None => return Ok(None),
            Some(s) => s,
        };

        let driver_state_register = status_ex.dsreg;
        DriverState::from_repr(driver_state_register as usize)
            .map(Option::Some)
            .ok_or_else(|| anyhow!("Unexpected dsreg: {driver_state_register}"))
    }
}
