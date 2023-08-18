use anyhow::{Context, Result};
use bincode::{Decode, Encode};
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, BufWriter, Seek, SeekFrom};
use std::mem::size_of;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

pub const CURRENT_VERSION: u8 = 0x01;

/// bincode 中实现的对 PathBuf 的序列化、反序列化代码，会将文件名按 UTF-8 对待
/// 这可能导致对非 UTF-8 文件名的反序列化出现错误. 因此底层使用 `Vec<u8>` 处理.
#[derive(Encode, Decode)]
pub struct D2fnPath {
    path: Vec<u8>,
}

impl Into<PathBuf> for D2fnPath {
    fn into(self) -> PathBuf {
        let os_path = OsString::from_vec(self.path);
        PathBuf::from(os_path)
    }
}

impl From<&Path> for D2fnPath {
    fn from(value: &Path) -> Self {
        let os_path = value.as_os_str();
        let path = os_path.as_bytes().to_vec();

        Self { path }
    }
}

#[derive(Encode, Decode, Default)]
pub struct Header {
    version: u8,
    offset: u8,
    count: usize,
}

#[derive(Encode, Decode)]
pub struct DuplicateFile {
    pub ino: u64,
    pub path: D2fnPath,
}

#[derive(Encode, Decode)]
pub struct DuplicateGroup {
    pub files: Vec<DuplicateFile>,
}

pub struct InventoryReader {
    reader: BufReader<File>,
    header: Header,

    read_count: usize,
}

pub struct InventoryWriter {
    writer: BufWriter<File>,
}

impl InventoryReader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let header = bincode::decode_from_reader(&mut reader, bincode::config::standard())
            .with_context(|| "reading header.".to_string())?;
        Ok(Self {
            reader,
            header,
            read_count: 0,
        })
    }

    pub fn total(&self) -> usize {
        self.header.count
    }
}

impl Iterator for InventoryReader {
    type Item = Result<DuplicateGroup>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.read_count < self.header.count {
            let group = bincode::decode_from_reader(&mut self.reader, bincode::config::standard()).map_err(Into::into);
            self.read_count += 1;
            Some(group)
        } else {
            None
        }
    }
}

impl InventoryWriter {
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let config = bincode::config::standard();
        bincode::encode_into_std_write(Header::default(), &mut writer, config)?;
        Ok(Self { writer })
    }

    pub fn export<T: Iterator<Item = DuplicateGroup>>(&mut self, groups: T) -> Result<()> {
        let mut count = 0usize;
        for group in groups {
            count += 1;
            bincode::encode_into_std_write(group, &mut self.writer, bincode::config::standard())?;
        }

        self.writer.seek(SeekFrom::Start(0))?;
        let new_header = Header {
            version: CURRENT_VERSION,
            offset: (4 + size_of::<usize>()) as u8,
            count,
        };
        bincode::encode_into_std_write(new_header, &mut self.writer, bincode::config::standard())?;
        Ok(())
    }
}
