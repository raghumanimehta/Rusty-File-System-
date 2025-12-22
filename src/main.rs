use fuser::{Filesystem, MountOption};
use std::env;

struct NullFS;

impl Filesystem for NullFS {}

const NUM_DATA_BLK: usize = 250_000; // 1GB FS, can be whatever, can even be made dynamic later

// 28 bytes starting at offset 0
// free inode bitmap can begin at offset 28 and inode table can follow immediately after
struct SuperBlock {
    ino_count: u32,
    blk_count: u32,
    free_blk_count: u32,
    free_ino_count: u32,
    super_blk_no: u32,
    mtime: u32,
    wtime: u32,
}

struct FreeBlockBitMap {
    map: [u8; NUM_DATA_BLK / 8 + 1],
}


fn main() {
    env_logger::init();
    let mountpoint = env::args_os().nth(1).unwrap();
    fuser::mount2(NullFS, mountpoint, &[MountOption::AutoUnmount]).unwrap();
}