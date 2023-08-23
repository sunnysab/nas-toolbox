fn main() {
    if cfg!(not(target_os = "freebsd")) {
        panic!("This crate now supports FreeBSD only, because some structure copied from mt.c in freebsd-src.");
    }
}
