use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

const DEFAULT_DATABASE_PATH: &str = "backup.db";

#[derive(Debug)]
pub struct Archive {
    /// Unique archive id
    id: u32,
    /// Tape id, refer to `id` in table `tape`
    tape: u8,
    /// Reported file number on the tape
    tape_file_index: u32,
    /// Archive size, in bytes
    size: u32,
    /// 32-byte blake3-hashed value
    hash: [u8; 32],
    /// The time when the file archived
    ts: u64,
    /// Flag, reserved
    flag: u32,
}

#[derive(Debug)]
pub struct FileOnDisk {
    id: u64,
    /// inode on filesystem. Note: it may conflict or be reused.
    inode: u64,
    /// file path
    path: String,
    /// flag
    flag: u32,
    /// Archive id, refer to `id` in table `archive`
    archive: u64,
    /// Version, which represented by a timestamp, is when the file scanned.
    version: u64,
}

#[derive(Debug)]
pub struct Tape {
    /// Tape number
    id: u16,
    /// Tape flag
    flag: u32,
    /// Some user-input description
    description: String,
}

pub struct Storage {
    /// SQLite connection
    conn: Connection,
}

impl Storage {
    fn create_default_database<P: AsRef<Path>>(path: P) -> Result<()> {
        let default_db_content = include_bytes!("../backup-template.db");

        std::fs::write(path, default_db_content).map(|_| ()).map_err(Into::into)
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            Self::create_default_database(path)
                .with_context(|| format!("failed to init default database at {}", path.display()))?;
        }

        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn append_file(&self, file: &FileOnDisk) -> Result<()> {
        let current_time = std::time::SystemTime::now();
        let duration = current_time.duration_since(std::time::UNIX_EPOCH).unwrap();
        let ts = duration.as_secs();

        self.conn
            .execute(
                "INSERT INTO file
            (inode, path, flag, archive, version)
            VALUES (?1, ?2, ?3, ?4, ?5);",
                (file.inode, &file.path, &file.flag, &file.archive, ts),
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn append_archive(&self, archive: &Archive) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO archive
            (tape, tape_file_index, size, hash, ts, flag)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                (
                    archive.tape,
                    archive.tape_file_index,
                    archive.size,
                    archive.hash,
                    archive.ts,
                    archive.flag,
                ),
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn create_tape(&self, flag: u32, description: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO tape
            (flag, description)
            VALUES (?1, ?2);",
                (flag, description),
            )
            .map(|_| ())
            .map_err(Into::into)
    }
}
