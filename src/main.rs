mod hash;
mod metadata;

use anyhow::{bail, Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

use filewalker::FileWalker;
use metadata::FileMetadata;

#[derive(Clone)]
struct File {
    path: PathBuf,
    metadata: FileMetadata,
}

fn into_file(entry: DirEntry) -> Result<File> {
    let path = entry.path();
    let flag = entry.file_type().map(|ft| ft.is_file()).unwrap_or(false);
    if !flag {
        let path = &path.display();
        bail!("unable to read file_type or it is not a file: {path}");
    }
    let metadata = entry
        .metadata()
        .map(metadata::convert_metadata)
        .with_context(|| format!("unable to query metadata to {}", path.display()))?;
    if metadata.size == 0 {
        bail!("file is empty");
    }
    Ok(File { path, metadata })
}

type FileExtension = u32;
type FileSize = u64;
type RecordIndex = usize;

/// A file extension like ".pdf" normally consists of numbers and letters.
/// I made a hash algorithm, mainly for extensions, generating integer hashes for them.
/// Note that "PDF" and "pdf" etc are same.
fn ext_hash(path: &Path) -> FileExtension {
    use std::os::unix::prelude::OsStrExt;

    let mut result = 0;
    if let Some(ext) = path.extension() {
        // We assume that there are only numbers and letters in ext.
        for x in ext.as_bytes() {
            let mut x = *x;

            if x & 64 != 0 {
                // letter
                x |= 32; // Make it lower case.
                result = result << 6 | x as u32;
            } else {
                // number
                x &= 15;
                result = result << 6 | x as u32;
            }
        }
    }
    result
}

enum PreviousScanned {
    Index(RecordIndex),
    Hash(HashSet<blake3::Hash>),
}

#[derive(Eq, PartialEq, Hash)]
struct ClassifyingKey(FileExtension, FileSize);

struct Duplicate<'a> {
    records: Vec<File>,

    inode_set: HashSet<u64>,
    /// (.pdf, 2MB) -> {a.pdf, b.pdf, c.pdf}
    /// (.pdf, 30M) -> {q.pdf, l.pdf}
    /// (.mp4, 400M) -> (1.mp4)
    set: HashMap<ClassifyingKey, PreviousScanned>,
    /// file hash -> [2, 4, ...]
    hash2files: HashMap<blake3::Hash, Vec<RecordIndex>>,

    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Duplicate<'a> {
    const DEFAULT_SIZE: usize = 1000_0000;

    fn new() -> Duplicate<'a> {
        Duplicate {
            records: Vec::with_capacity(Self::DEFAULT_SIZE),
            inode_set: HashSet::with_capacity(Self::DEFAULT_SIZE),
            set: HashMap::with_capacity(Self::DEFAULT_SIZE),
            hash2files: HashMap::with_capacity(Self::DEFAULT_SIZE),
            _marker: Default::default(),
        }
    }

    fn append_record(&mut self, file: File) -> RecordIndex {
        let index = self.records.len();
        self.records.push(file);

        index
    }

    fn push(&mut self, file: File) -> Result<()> {
        let ino = file.metadata.ino;
        let path = file.path.clone();
        let extension = ext_hash(&file.path);
        let size = file.metadata.size;

        if self.inode_set.contains(&ino) {
            // 忽略已经记录过的文件
            return Ok(());
        }
        // 先记一个 ino
        // 如果当前文件之前（t时刻）去重过, 那么它只会被添加进来一次, 且, 自那次去重后新产生的、与它重复的文件会被识别到.
        // 如果没去重过也不影响, 未去重时他们的 ino 不同.
        self.inode_set.insert(ino);

        // 将当前文件信息存起, 便于后续比对.
        let index = self.append_record(file);
        let key = ClassifyingKey(extension, size);
        if let Some(previous_result) = self.set.get_mut(&key) {
            // 存在与当前文件相同扩展名和大小的文件，且 inode 不同.
            // 需要通过哈希值进行最终的判断
            let hash = hash::checksum_file(&path, hash::MODE_HEAD_1M)?;
            // 这里使用了 PreviousScanned 结构. 由于估计存在大量非重复文件, 对于第一次出现满足某个 (ext, size)
            // 组合的文件只记录其下标, 等到第二次遇到该组合时再计算其哈希值, 以减少计算量
            if let PreviousScanned::Index(previous_index) = previous_result {
                let file = &self.records[*previous_index];
                let previous_hash = hash::checksum_file(&file.path, hash::MODE_HEAD_1M)?;

                let mut set_of_file_hash_in_ext_size = HashSet::new();
                set_of_file_hash_in_ext_size.insert(previous_hash);

                let i = *previous_index;
                *previous_result = PreviousScanned::Hash(set_of_file_hash_in_ext_size);

                // 把之前扫描中遇到的这个文件, 它的哈希值不存在于 hash2files 中, 可以加进去
                // 这可能导致最终结果里 hash2files 出现一些 value.len() == 1 的键值对, 滤去即可
                self.hash2files.insert(hash, vec![i]);
            }

            // 现在 PreviousScanned 一定记录了一个哈希值的集合
            // 如果当前文件是重复出现的, 即 hash 出现重复, 那么 set 和 hash2files 中已经存在这个哈希值了, 需要在 hash2files 登记一下
            // 如果当前文件第一次出现, 需要将 hash 添加到 set 中, 并在 hash2files 中记录 （后面没有机会记录了）
            if let PreviousScanned::Hash(set) = previous_result {
                // 依上述分析, 直接添加
                set.insert(hash);
                // 在 hash2files 里记录一下
                if let Some(duplicate_file_list) = self.hash2files.get_mut(&hash) {
                    duplicate_file_list.push(index);
                } else {
                    self.hash2files.insert(hash, vec![index]);
                }
            } // 不需要 else, 因为已经保证 PreviousScanned 为 Hash
        } else {
            // 若头一次遇到 (ext, size)
            let scanned_result = PreviousScanned::Index(index);
            self.set.insert(key, scanned_result);
        }

        Ok(())
    }

    fn map_record_vec(&'a self, v: &Vec<RecordIndex>) -> Vec<&'a File> {
        let mut result = Vec::new();

        for index in v {
            result.push(&self.records[*index]);
        }
        result
    }

    fn get_groups(&'a self) -> impl Iterator<Item = Vec<&'a File>> {
        self.hash2files
            .iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|(_, record_vec)| self.map_record_vec(record_vec))
    }
}

fn main() {
    let path = Path::new("/home/sunnysab");
    let mut duplicate = Duplicate::new();

    // TODO: Walker 需要支持跳过隐藏的文件夹.
    let walker = FileWalker::open(&path).unwrap();
    for item in walker {
        let file = match item.map_err(Into::into).and_then(into_file) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("warning: {e}");
                continue;
            }
        };

        let path = file.path.clone();
        if let Err(e) = duplicate.push(file) {
            eprintln!("unable to add {}: {}", path.display(), e);
        }
    }

    // 统计结果
    let mut count = 1;
    for file_group in duplicate.get_groups() {
        println!("group {count}:");
        count += 1;

        for file in file_group {
            println!(" - {}", file.path.display());
        }
    }
}
