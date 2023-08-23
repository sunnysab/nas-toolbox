# Tape

[ENGLISH VERSION](README_EN.md)

Tape 是一个使用 Rust 语言编写的、用于操作 SCSI 磁带机的库，它移植了 FreeBSD 中 mt 命令的大部分功能。部分代码从 
`freebsd-src/usr.bin/mt/mt.c` 翻译而来。由于 FreeBSD 中 的磁带设备使用 sa(4) 驱动，而 Linux 下由 st(4) 驱动，其数据结构的定义有细
微差别，因此这个 crate 仅提供了对 FreeBSD 系统的支持。

sa 提供了一组磁带设备接口。假定设备序号为 0，则设备依次为 `/dev/nsa0`、`/dev/sa0`、`/dev/esa0`。后两者在读写后会自动倒带或弹出，因此不
建议使用。

Tape 仅在以下环境中进行了测试：

- FreeBSD 13.2
- Rust 1.71.1
- HP Ultrium 6-SCSI


## 导入

由于 Tape 处于一个工作区中，并且它暂时没有被发布到 Crates.io，要想引用它，需要在 `Cargo.toml` 中添加：

```toml
tape = { git = "https://github.com/sunnysab/nas-toolbox", package = "tape" }
```


## 支持的操作

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