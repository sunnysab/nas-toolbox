use anyhow::{Context, Result};
use bincode::{Decode, Encode};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
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

impl From<D2fnPath> for PathBuf {
    fn from(value: D2fnPath) -> Self {
        let os_path = OsString::from_vec(value.path);
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
    count: u32,
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
    buffer: Vec<u8>,

    header: Header,
    read_count: u32,
}

pub struct InventoryWriter {
    buffer: Vec<u8>,
    writer: BufWriter<File>,
}

impl InventoryReader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let buffer = vec![0u8; 1024 * 1024];
        let mut reader = BufReader::new(file);

        let header = Self::read_header(&mut reader).with_context(|| "reading header.".to_string())?;
        Ok(Self {
            reader,
            buffer,
            header,
            read_count: 0,
        })
    }

    pub fn total(&self) -> usize {
        self.header.count as usize
    }

    fn read_header<R: BufRead>(mut reader: R) -> Result<Header> {
        let version = reader.read_u8()?;
        let offset = reader.read_u8()?;
        let count = reader.read_u32::<LittleEndian>()?;

        Ok(Header { version, offset, count })
    }

    fn decode<D: Decode + Sized, R: BufRead>(mut reader: R, buf: &mut [u8]) -> Result<D> {
        let size = reader.read_u32::<LittleEndian>()?;

        reader.read_exact(&mut buf[..size as usize])?;
        let (data, _) = bincode::decode_from_slice(&buf[..size as usize], bincode::config::standard())?;
        Ok(data)
    }
}

impl Iterator for InventoryReader {
    type Item = Result<DuplicateGroup>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.read_count < self.header.count {
            let result = Self::decode(&mut self.reader, &mut self.buffer);

            self.read_count += 1;
            Some(result)
        } else {
            None
        }
    }
}

impl InventoryWriter {
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::create(path)?;
        let buffer = vec![0u8; 1024 * 1024];
        let mut writer = BufWriter::new(file);

        Self::write_header(&mut writer, &Header::default())?;
        Ok(Self { writer, buffer })
    }

    fn write_header<W: Write>(writer: &mut W, header: &Header) -> Result<()> {
        writer.write_u8(header.version)?;
        writer.write_u8(header.offset)?;
        writer.write_u32::<LittleEndian>(header.count)?;
        Ok(())
    }

    fn encode<D: Encode, W: Write>(val: D, writer: &mut W, buf: &mut [u8]) -> Result<()> {
        let size = bincode::encode_into_slice(val, buf, bincode::config::standard())?;

        writer.write_u32::<LittleEndian>(size as u32)?;
        writer.write_all(&buf[..size])?;
        Ok(())
    }

    pub fn export<T: Iterator<Item = DuplicateGroup>>(&mut self, groups: T) -> Result<()> {
        let mut count = 0u32;
        for group in groups {
            count += 1;
            Self::encode(group, &mut self.writer, &mut self.buffer)?;
        }

        let new_header = Header {
            version: CURRENT_VERSION,
            offset: (2 + size_of::<usize>()) as u8,
            count,
        };
        self.writer.seek(SeekFrom::Start(0))?;
        Self::write_header(&mut self.writer, &new_header)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::inventory::{D2fnPath, DuplicateFile, DuplicateGroup, InventoryReader, InventoryWriter};
    use std::path::{Path, PathBuf};

    fn generate_test_data() -> Vec<DuplicateGroup> {
        let file1 = "file1.txt".as_bytes().to_vec();
        let file2 = "file2.txt".as_bytes().to_vec();
        let file3 = "中文字符.txt".as_bytes().to_vec();
        let file4 = "符号(x).txt".as_bytes().to_vec();
        let file5 = "file5\0.txt".as_bytes().to_vec();

        vec![
            DuplicateGroup {
                files: vec![
                    DuplicateFile {
                        ino: 1,
                        path: D2fnPath { path: file1 },
                    },
                    DuplicateFile {
                        ino: 2,
                        path: D2fnPath { path: file2 },
                    },
                    DuplicateFile {
                        ino: 3,
                        path: D2fnPath { path: file3 },
                    },
                ],
            },
            DuplicateGroup {
                files: vec![
                    DuplicateFile {
                        ino: 4,
                        path: D2fnPath { path: file4 },
                    },
                    DuplicateFile {
                        ino: 5,
                        path: D2fnPath { path: file5 },
                    },
                ],
            },
        ]
    }

    #[test]
    fn test_d2fn_path() {
        let path = Path::new("./test-file");
        let dataset = generate_test_data();

        let mut writer = InventoryWriter::create(path).unwrap();
        writer.export(dataset.into_iter()).unwrap();
        drop(writer);

        let reader = InventoryReader::open(path).unwrap();
        println!("len(groups) = {}", reader.header.count);
        for group in reader {
            let group = group.unwrap();
            for item in group.files {
                let path = Into::<PathBuf>::into(item.path);
                println!("({}): {}", item.ino, path.display());
            }
        }
        std::fs::remove_file("./test-file").unwrap();
    }
}
