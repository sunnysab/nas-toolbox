//! In order to compare more than two files, we still need checksum.

use std::fs::File;
use std::io::{IsTerminal, Read};

use anyhow::Result;
use std::path::Path;

pub const MODE_HEAD_1M: CompareMode = CompareMode::Part(1024 * 1024);

pub enum CompareMode {
    Full,
    Part(usize),
}

pub fn checksum_file<P: AsRef<Path>>(path: P, mode: CompareMode) -> Result<blake3::Hash> {
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut file = File::options().read(true).write(false).open(&path)?;

    let mut hasher = blake3::Hasher::new();
    let mut hashed_size = 0usize;
    let compare_size = if let CompareMode::Part(compare_size) = mode {
        compare_size
    } else {
        usize::MAX
    };
    while !file.is_terminal() && hashed_size < compare_size {
        let len = file.read(&mut buffer)?;

        hasher.update(&buffer[..len]);
        hashed_size += len;
    }

    let result = hasher.finalize();
    Ok(result)
}
