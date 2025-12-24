use fuser::{Filesystem, MountOption};
use std::env;
use std::fs::FileType;
use bitvec::prelude::*;


struct NullFS;

impl Filesystem for NullFS {}

// const BLOCK_SIZE: u64 = 4096;  Don't remember why I declared this; maybe because of reference 
const BITMAP_SIZE_BYTES: usize = 4096; 
const NUM_DIRECT_PTR: usize = 12;

pub struct Inode {
    pub ino_id: u64,            // inode number
    pub size: u64,              // file size 
    pub blocks: u64,            // num blocks allocated 
    pub mtime_secs: i64,        // Easier to save to disk than SystemTime. Ignored the atime and ctime for now. 
    pub kind: FileType,  
    pub perm: u16, 
    pub direct_blks: [u32; NUM_DIRECT_PTR], 
    pub indirect_blk: u32,
    pub dbl_indirect_blk: u32,
    pub tri_indirect_blk: u32, 
}

pub struct InodeBitmap {
    pub map: BitArray<[u8; BITMAP_SIZE_BYTES], Lsb0>
}

impl InodeBitmap {

    pub fn new() -> Self {
        Self {
            map: BitArray::ZERO
        } 
    }

    pub fn find_first_free(&self) -> Option<usize> {
        self.map.first_zero()
    }

}

fn main() {
    env_logger::init();
    let mountpoint = env::args_os().nth(1).unwrap();
    fuser::mount2(NullFS, mountpoint, &[MountOption::AutoUnmount]).unwrap();
}