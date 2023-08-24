use anyhow::{Context, Result};
use std::io::{Read, Seek, Write};
use std::os::fd::FromRawFd;
use tape::{LocationBuilder, TapeDevice};

fn main() -> Result<()> {
    let tape = TapeDevice::open("/dev/nsa0")?;
    tape.rewind().expect("unable to rewind the tape.");

    let fd = tape.fd();
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let mut buffer = [0u8; 512];

    for v in 0..8 {
        for i in 0..512 {
            buffer[i] = v;
        }
        let pos = tape.read_scsi_pos()?;
        println!("pos = {pos}");
        let count = file.write(&buffer).with_context(|| format!("when write {v}"))?;
        println!("count = {count}");

        if v % 2 == 0 {
            tape.write_eof(1).with_context(|| format!("write eof"))?;
        }
    }

    tape.rewind()?;
    for _ in 0..8 {
        for i in 0..512 {
            buffer[i] = 0;
        }
        let pos = tape.read_scsi_pos()?;
        println!("pos = {pos}");

        let actual_read = file.read(&mut buffer)?;
        println!("({}) {:?}", actual_read, &buffer[..actual_read]);
    }
    Ok(())
}
