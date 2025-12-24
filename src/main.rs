use fuser::{Filesystem, MountOption, FileType};
use std::env;
use bitvec::prelude::*;


struct NullFS;
impl Filesystem for NullFS {}

const BLK_SIZE: usize = 4096;
const FS_SIZE: usize = 512usize * (1usize << 30); 
const NUM_DATA_BLKS: usize = FS_SIZE / BLK_SIZE ;

const DATA_BLK_BITMAP_BYTES: usize = (NUM_DATA_BLKS + 7) / 8;

const BITMAP_SIZE_BYTES: usize = 4096;
const NUM_DIRECT_PTR: usize = 12;

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

trait FreeObjectBitmap<const N: usize> {
    fn map(&self) -> &BitArray<[u8; N], Lsb0>;

    fn find_first_free(&self) -> Option<usize> {
        self.map().first_zero()
    }
}

struct FreeBlockBitmap {
    map: BitArray<[u8; DATA_BLK_BITMAP_BYTES], Lsb0>,
}

impl FreeBlockBitmap {
    fn new() -> Self {
        Self { map: BitArray::ZERO }
    }
}

impl FreeObjectBitmap<DATA_BLK_BITMAP_BYTES> for FreeBlockBitmap {
    fn map(&self) -> &BitArray<[u8; DATA_BLK_BITMAP_BYTES], Lsb0> {
        &self.map
    }
}

struct Inode {
    ino_id: u64,            // inode number
    size: u64,              // file size 
    blocks: u64,            // num blocks allocated 
    mtime_secs: i64,        // Easier to save to disk than SystemTime. Ignored the atime and ctime for now. 
    kind: FileType,  
    perm: u16, 
    direct_blks: [u32; NUM_DIRECT_PTR], 
    indirect_blk: u32,
    dbl_indirect_blk: u32,
    tri_indirect_blk: u32, 
}

struct InodeBitmap {
    map: BitArray<[u8; BITMAP_SIZE_BYTES], Lsb0>,
}

impl InodeBitmap {
    fn new() -> Self {
        Self { map: BitArray::ZERO }
    }
}

impl FreeObjectBitmap<BITMAP_SIZE_BYTES> for InodeBitmap {
    fn map(&self) -> &BitArray<[u8; BITMAP_SIZE_BYTES], Lsb0> {
        &self.map
    }
}

fn main() {
    env_logger::init();
    let mountpoint = env::args_os().nth(1).unwrap();
    fuser::mount2(NullFS, mountpoint, &[MountOption::AutoUnmount]).unwrap();
}