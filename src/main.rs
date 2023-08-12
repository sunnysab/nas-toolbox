mod duplicate;
mod hash;
mod metadata;

use anyhow::Result;
use duplicate::Duplicate;
use std::path::Path;

fn main() -> Result<()> {
    let path = Path::new("/home/sunnysab");
    let mut duplicate = Duplicate::new(&path);

    // 统计结果
    let mut count = 1;
    for file_group in duplicate.discover()? {
        println!("group {count}:");
        count += 1;

        for file in file_group {
            println!(" - {}", file.path.display());
        }
    }
    Ok(())
}
