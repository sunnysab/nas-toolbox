#[derive(Clone)]
pub struct FileMetadata {
    /// Inode number
    pub ino: u64,
    /// Number of hard links to file
    pub link_count: u64,
    /// File size (in bytes)
    pub size: u64,
    /// Allocated blocks, in 512-byte units
    pub blocks: u64,
}

#[cfg(target_os = "unix")]
pub fn convert_metadata(metadata: std::fs::Metadata) -> FileMetadata {
    use std::os::unix::fs::MetadataExt;

    let ino = metadata.ino();
    let link_count = metadata.nlink();
    let size = metadata.size();
    let blocks = metadata.blocks();

    FileMetadata {
        ino,
        link_count,
        size,
        blocks,
    }
}

#[cfg(target_os = "linux")]
pub fn convert_metadata(metadata: std::fs::Metadata) -> FileMetadata {
    use std::os::linux::fs::MetadataExt;

    let ino = metadata.st_ino();
    let link_count = metadata.st_nlink();
    let size = metadata.st_size();
    let blocks = metadata.st_blocks();

    FileMetadata {
        ino,
        link_count,
        size,
        blocks,
    }
}
