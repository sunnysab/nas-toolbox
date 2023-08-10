use std::path::Path;
use filewalker::FileWalker;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let path = Path::new("/home/sunnysab");
    let walker = FileWalker::open(path).unwrap();

    for item in walker {
        let item = item.unwrap();

        println!("{}", item.display());
        sleep(Duration::from_secs(2));
    }
}
