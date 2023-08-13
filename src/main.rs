mod duplicate;
mod hash;
mod metadata;

use anyhow::Result;
use duplicate::{Duplicate, File};
use std::ffi::OsStr;
use std::path::Path;

const DEFAULT_EXT_FILTER: [&str; 44] = [
    "pdf", "mdx", "epub", "djvu", "xps", // Document
    "class", "exe", "dll", "so", "bin", "apk", // Build craft
    "zip", "rar", "7z", "iso", "tar", "tgz", "bak", // Archive
    "mp3", "wav", "flac", "ape", "ogg", "aac", // Music
    "mp4", "rm", "mkv", "avi", "mov", "wmv", "flv", "webm", "rmvb", "f4v", "mpg", "mpeg",
    "ts", // Video
    "jpg", "bmp", "jpeg", "gif", "png", "webp",
    "tiff", // Picture. Note: Please not modify these pictures.
];

fn filter(ext_whitelist: &[&OsStr], file: &File) -> bool {
    for predefined_ext in ext_whitelist {
        if let Some(this_ext) = file.path.extension() {
            if this_ext == *predefined_ext {
                return true;
            }
        }
    }
    false
}

fn main() -> Result<()> {
    let path = Path::new("/home/sunnysab");
    let ext_set = DEFAULT_EXT_FILTER
        .iter()
        .map(|x| OsStr::new(x))
        .collect::<Vec<_>>();
    let mut duplicate =
        Duplicate::new(path).custom_filter(move |file| filter(ext_set.as_slice(), file));

    duplicate.discover()?;

    // 统计结果
    let mut count = 1;
    for file_group in duplicate.result() {
        println!("group {count}:");
        count += 1;

        for file in file_group {
            println!(" - {}", file.path.display());
        }
    }
    Ok(())
}
