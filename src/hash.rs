//! In order to compare more than two files, we still need checksum.

use std::fs::File;
use std::io::Read;

use anyhow::Result;
use std::path::Path;

pub const MODE_HEAD_1M: CompareMode = CompareMode::Part(1024 * 1024);

pub enum CompareMode {
    Full,
    Part(usize),
}

pub fn checksum_file<P: AsRef<Path>>(path: P, mode: CompareMode) -> Result<blake3::Hash> {
    const CHUNK_SIZE: usize = 1024 * 1024;
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut file = File::options().read(true).write(false).open(&path)?;

    let mut hasher = blake3::Hasher::new();
    let mut hashed_size = 0usize;
    let compare_size = if let CompareMode::Part(compare_size) = mode {
        compare_size
    } else {
        usize::MAX
    };

    // 假定
    // 1. 不存在哈希碰撞
    // 2. 文件是常规文件, 不存在 file hole.
    // 这个假设很重要, 因为它避免了两个不同的文件计算出同一哈希值
    // 由于不知道文件大小, 因此读完 expected size 或读取出现 len == 0 后停止.
    while let Ok(len) = file.read(&mut buffer) {
        if len == 0 {
            break;
        }
        let current_hash_len = std::cmp::min(compare_size - hashed_size, CHUNK_SIZE);
        hasher.update(&buffer[..current_hash_len]);
        hashed_size += len;

        if hashed_size == compare_size {
            break;
        }
    }

    let result = hasher.finalize();
    Ok(result)
}
