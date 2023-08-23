# Tape

[中文版本](README.md)

Tape is a library written in the Rust programming language for manipulating SCSI tape drives. It has ported most of the
functionalities from the "mt" command in FreeBSD. Some of the code has been translated from `freebsd-src/usr.bin/mt/mt.c`.
Since tape devices in FreeBSD use the sa(4) driver, while in Linux they use the st(4) driver, there are slight differences 
in the definition of their data structures. Therefore, this crate only provides support for FreeBSD systems.

sa provides a set of interfaces for tape devices. Assuming the device number is 0, the devices are `/dev/nsa0`, `/dev/sa0`,
and `/dev/esa0` in sequential order. The last two automatically rewind or eject after reading or writing, so it is not 
recommended to use them.

Tape has only been tested in the following environments:

- FreeBSD 13.2
- Rust 1.71.1
- HP Ultrium 6-SCSI

## Import

Since Tape is in a workspace and it hasn't been published to Crates.io yet, to reference it, add the following to
'Cargo.toml' :


```toml
tape = { git = "https://github.com/sunnysab/nas-toolbox", package = "tape" }
```

## Supports

- [x] mt weof / weofi
- [x] mt smk
- [x] mt fsf / bsf
- [x] mt fsr / bsr
- [x] mt fss / bss
- [x] mt erase
- [ ] mt rdhpos / sethpos
- [x] mt rdspos / setspos
- [x] mt rewind
- [x] mt offline
- [x] mt load
- [x] mt retension
- [x] mt ostatus / status
- [x] mt errstat
- [x] mt geteotmodel / seteotmodel
- [x] mt eod
- [x] mt rblim
- [x] mt blocksize
- [ ] mt param
- [x] mt protect (query only)
- [x] mt locate
- [x] mt comp
- [x] mt getdensity / density