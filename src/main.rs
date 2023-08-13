mod duplicate;
mod hash;
mod metadata;

use crate::duplicate::DefaultFilter;
use anyhow::Result;
use duplicate::Duplicate;
use std::path::Path;

fn main() -> Result<()> {
    let path = Path::new("/home/sunnysab");
    let mut duplicate = Duplicate::new(path).custom_filter(DefaultFilter::new());

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
