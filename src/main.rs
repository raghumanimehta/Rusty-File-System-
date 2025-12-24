use fuser::{Filesystem, MountOption};
use std::env;
use std::fs::FileType;
use bitvec::prelude::*;


struct NullFS;

impl Filesystem for NullFS {}

const NUM_DATA_BLKS: usize = 250_000; // 1GB FS, can be whatever, can even be made dynamic later

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

trait FreeObjectBitMap {
    fn new() -> Self {
        Self {
            bitmap: BitArray::ZERO
        }
    }

    fn find_first_free(&self) -> Option<usize> {
        self.map.first_zero()
    }
}

struct FreeBlockBitMap {
    bitmap: BitArray<[u8; NUM_DATA_BLKS], Lsb0>,
}

impl FreeObjectBitMap for FreeBlockBitMap {}


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