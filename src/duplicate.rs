use anyhow::{bail, Context, Result};

use blake3::Hash;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use crate::hash::{checksum_file, CompareMode};
use crate::metadata::{convert_metadata, FileMetadata};
use filewalker::FileWalker;

const DEFAULT_EXT_FILTER: [&str; 44] = [
    "pdf", "mdx", "epub", "djvu", "xps", // Document
    "class", "exe", "dll", "so", "bin", "apk", // Build craft
    "zip", "rar", "7z", "iso", "tar", "tgz", "bak", // Archive
    "mp3", "wav", "flac", "ape", "ogg", "aac", // Music
    "mp4", "rm", "mkv", "avi", "mov", "wmv", "flv", "webm", "rmvb", "f4v", "mpg", "mpeg", "ts", // Video
    "jpg", "bmp", "jpeg", "gif", "png", "webp", "tiff", // Picture. Note: Please not modify these pictures.
];

#[derive(Clone)]
pub struct File {
    pub path: PathBuf,
    pub metadata: FileMetadata,
}

impl TryFrom<DirEntry> for File {
    type Error = anyhow::Error;

    fn try_from(value: DirEntry) -> std::result::Result<Self, Self::Error> {
        let path = value.path();
        let metadata = value
            .metadata()
            .map(convert_metadata)
            .with_context(|| format!("unable to query metadata to {}", path.display()))?;
        if metadata.size == 0 {
            bail!("file is empty");
        }
        Ok(File { path, metadata })
    }
}

type FileExtension = u32;
type FileSize = u64;
type RecordIndex = usize;

pub trait ScanFilter {
    fn filter(&self, file: &File) -> bool;
}

pub struct NoFilter;

impl ScanFilter for NoFilter {
    fn filter(&self, _file: &File) -> bool {
        true
    }
}

pub struct DefaultFilter<'a> {
    ext: Vec<&'a OsStr>,
}

impl DefaultFilter<'_> {
    pub fn new() -> Self {
        let ext_set = DEFAULT_EXT_FILTER.iter().map(OsStr::new).collect::<Vec<_>>();
        Self { ext: ext_set }
    }

    pub fn ext_set() -> &'static [&'static str] {
        &DEFAULT_EXT_FILTER
    }
}

impl ScanFilter for DefaultFilter<'_> {
    fn filter(&self, file: &File) -> bool {
        for predefined_ext in &self.ext {
            if let Some(this_ext) = file.path.extension() {
                if this_ext == *predefined_ext {
                    return true;
                }
            }
        }
        false
    }
}

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

pub struct Duplicate<'a, F: ScanFilter> {
    path: PathBuf,

    records: Vec<File>,
    inode_set: HashSet<u64>,
    /// (.pdf, 2MB) -> {a.pdf, b.pdf, c.pdf}
    /// (.pdf, 30M) -> {q.pdf, l.pdf}
    /// (.mp4, 400M) -> (1.mp4)
    set: HashMap<ClassifyingKey, PreviousScanned>,
    /// file hash -> [2, 4, ...]
    hash2files: HashMap<blake3::Hash, Vec<RecordIndex>>,
    full_hash2files: HashMap<blake3::Hash, Vec<RecordIndex>>,

    filter: F,

    status_channel: Option<Sender<StatusReport>>,
    status_report_step: usize,
    status: StatusReport,

    _marker: std::marker::PhantomData<&'a ()>,
}

#[derive(Default)]
pub struct StatusReport {
    pub scanned: usize,
    pub duplicated: usize,

    pub last_file: String,
}

impl<'a> Duplicate<'a, NoFilter> {
    const DEFAULT_SIZE: usize = 100_0000;

    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();

        Duplicate {
            path,
            records: Vec::with_capacity(Self::DEFAULT_SIZE),
            inode_set: HashSet::with_capacity(Self::DEFAULT_SIZE),
            set: HashMap::with_capacity(Self::DEFAULT_SIZE),
            hash2files: HashMap::with_capacity(Self::DEFAULT_SIZE),
            full_hash2files: HashMap::new(),
            filter: NoFilter,
            status_channel: None,
            status_report_step: usize::MAX,
            status: Default::default(),
            _marker: Default::default(),
        }
    }
}

impl<'a, F: ScanFilter> Duplicate<'a, F> {
    pub fn custom_filter<G: ScanFilter>(self, filter: G) -> Duplicate<'a, G> {
        let Duplicate {
            path,
            records,
            inode_set,
            set,
            hash2files,
            ..
        } = self;
        Duplicate {
            path,
            records,
            inode_set,
            set,
            hash2files,
            filter,
            full_hash2files: HashMap::new(),
            status_channel: None,
            status_report_step: 0,
            status: Default::default(),
            _marker: Default::default(),
        }
    }

    pub fn enable_status_channel(&mut self, step: usize) -> Receiver<StatusReport> {
        assert!(step > 0);

        self.status_report_step = step;

        let (tx, rx) = mpsc::channel();
        self.status_channel = Some(tx);
        rx
    }

    fn append_record(&mut self, file: File) -> RecordIndex {
        let index = self.records.len();
        self.records.push(file);

        index
    }

    fn push(&mut self, file: File, compare_size: usize) -> Result<()> {
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
            let hash = checksum_file(path, CompareMode::Part(compare_size))?;
            // 这里使用了 PreviousScanned 结构. 由于估计存在大量非重复文件, 对于第一次出现满足某个 (ext, size)
            // 组合的文件只记录其下标, 等到第二次遇到该组合时再计算其哈希值, 以减少计算量
            if let PreviousScanned::Index(previous_index) = previous_result {
                let previous_file = &self.records[*previous_index];
                let previous_hash = checksum_file(&previous_file.path, CompareMode::Part(compare_size))?;

                let mut set_of_file_hash_in_ext_size = HashSet::new();
                set_of_file_hash_in_ext_size.insert(previous_hash);

                let i = *previous_index;
                *previous_result = PreviousScanned::Hash(set_of_file_hash_in_ext_size);

                // 把之前扫描中遇到的这个文件, 它的哈希值不存在于 hash2files 中, 可以加进去
                // 这可能导致最终结果里 hash2files 出现一些 value.len() == 1 的键值对, 滤去即可
                self.hash2files.insert(previous_hash, vec![i]);
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
                    self.status.duplicated += 1;
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

    pub fn result(&'a self) -> impl Iterator<Item = Vec<&'a File>> {
        let group_set1 = self
            .hash2files
            .iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|(_, record_vec)| self.map_record_vec(record_vec));

        let group_set2 = self
            .full_hash2files
            .iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|(_, record_vec)| self.map_record_vec(record_vec));

        group_set1.chain(group_set2)
    }

    pub fn discover(&mut self, compare_size: usize) -> Result<()> {
        let walker = FileWalker::open(&self.path)
            .with_context(|| format!("failed to read start directory: {}", self.path.display()))?
            .file_only(true)
            .filter_hidden_items(true)
            .flatten();

        for item in walker {
            if let Ok(file) = File::try_from(item) {
                let path = file.path.clone();
                self.status.scanned += 1;
                // 报告当前扫描进度
                if self.status_channel.is_some() && self.status.scanned % self.status_report_step == 0 {
                    if let Some(channel) = &self.status_channel {
                        let path = path.to_string_lossy().to_string();
                        let report = StatusReport {
                            last_file: path,
                            ..self.status
                        };
                        let _ = channel.send(report);
                    }
                }

                if !self.filter.filter(&file) {
                    continue;
                }

                if let Err(e) = self.push(file, compare_size) {
                    eprintln!("unable to add {}: {}", path.display(), e);
                }
            };
        }
        Ok(())
    }

    pub fn verify(&mut self) -> Result<usize> {
        let mut conflict_count = 0usize;

        for (_, vec) in self.hash2files.iter_mut() {
            if vec.len() == 1 {
                continue;
            }

            // vec 是一个文件下标集合, 现在需要找到对应的 File 结构, 并计算其文件哈希值.
            // 按计算结果, 验证文件是否重复.
            let mut full_checksum_map: HashMap<Hash, Vec<RecordIndex>> = HashMap::new();
            for i in vec.iter() {
                let file = &self.records[*i];
                let full_checksum =
                    checksum_file(&file.path, CompareMode::Full).with_context(|| format!("read {}", file.path.display()))?;

                if let Some(same_checksum_files) = full_checksum_map.get_mut(&full_checksum) {
                    same_checksum_files.push(*i);
                } else {
                    full_checksum_map.insert(full_checksum, vec![*i]);
                }
            }

            // 如果真的出现了：前 compare_size 大小相同, 但完整的文件不同的情况（针对存档文件少见）
            // 注意，这里不考虑哈希碰撞，即：默认只有部分哈希相同，完整的哈希才有可能相同.
            if full_checksum_map.len() > 1 {
                vec.clear();
                conflict_count += full_checksum_map.len();

                for (full_checksum, mut array) in full_checksum_map.into_iter() {
                    if let Some(old_array) = self.full_hash2files.get_mut(&full_checksum) {
                        old_array.append(&mut array);
                    } else {
                        self.full_hash2files.insert(full_checksum, array);
                    }
                }
            }
        }
        Ok(conflict_count)
    }
}
