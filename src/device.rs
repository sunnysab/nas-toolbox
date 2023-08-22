mod status;

pub use status::{Density, DriverState, TapeStatus};

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
    /// Enable controller cach
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
