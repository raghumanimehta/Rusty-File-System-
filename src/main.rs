use bitvec::prelude::*;
use fuser::{FileType, Filesystem, MountOption};
use log::{debug, error, info, log_enabled, Level};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

struct NullFS;
impl Filesystem for NullFS {}

// This is the total capacity of the backing storage for the file system
// this includes the space used for the FSMetadata, free object bitmaps, and file data and metadata
const FS_SIZE_BYTES: u64 = 1u64 * (0b1 << 30) as u64; // 1 GB
const BLK_SIZE_BYTES: u64 = 4096u64;
// 0 -> FSMetadata, 1->InodeBitmap, 2 -> Freeblock bitmap 
const RESERVED_DATA_BLKS: u32 = 3;
const NUM_DATA_BLKS: u32 = (FS_SIZE_BYTES / BLK_SIZE_BYTES) as u32;
const FREE_BLK_BMAP_SIZE_BYTES: usize = ((NUM_DATA_BLKS + 7) / 8) as usize;

// Inodes
const MAX_NUM_INODES: u32 = 10;
const RESERVED_INODES: u32 = 2; // 0: null inode, 1: root
const FREE_INODE_BMAP_SIZE_BYTES: usize = ((MAX_NUM_INODES + 7) / 8) as usize;
const NUM_INO_DIRECT_PTR: usize = 12;
const INVALID_PTR: u32 = 0;

// free inode bitmap can begin right after this struct and inode table can follow immediately after
struct FSMetadata {
    ino_count: u32,
    blk_count: u32,
    free_blk_count: u32,
    free_ino_count: u32,
    super_blk_no: u32,
    mtime: u64,
    wtime: u64,
}

impl Default for FSMetadata {
    fn default() -> Self {
        Self {
            ino_count: MAX_NUM_INODES,
            blk_count: NUM_DATA_BLKS,
            free_blk_count: NUM_DATA_BLKS - RESERVED_DATA_BLKS,
            free_ino_count: MAX_NUM_INODES - RESERVED_INODES,
            super_blk_no: 0,
            mtime: 0,
            wtime: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct Block {
    data: [u8; BLK_SIZE_BYTES as usize],
}

#[derive(Debug)]
enum BitMapError {
    RestrictedEntry, 
    AlreadyAlloced,
    AlreadyFree
}
    

trait FreeObjectBitmap<const N: usize> {
    const RESERVED: usize; 
    const MAX: usize; 

    fn map(&mut self) -> &mut BitArray<[u8; N], Lsb0>;

    fn find_first_free(&mut self) -> Option<usize> {
        match self.map().first_zero() {
            Some(idx) => {
                if idx < Self::RESERVED {
                    None
                } else {
                    Some(idx)
                }
            }
            None => None,
        }
    }

    fn set_alloc(&mut self, idx: usize) -> Result<(), BitMapError> {
        if idx < Self::RESERVED || idx > Self::MAX {
            return Err(BitMapError::RestrictedEntry)
        } 
        if self.map()[idx] == true {
            error!("The index is already alloced, no change")
            return Err(BitMapError::AlreadyAlloced) 
        } else { 
            self.map().set(idx, true);
            Ok(())
        }
    }

    fn set_free(&mut self, idx: usize) -> Result<(), BitMapError> {
        if idx < Self::RESERVED || idx > Self::MAX {
            return Err(BitMapError::RestrictedEntry)
        } 
        if self.map()[idx] == false {
            error!("The index is already free, no change")
            return Err(BitMapError::AlreadyFree) 
        } else { 
            self.map().set(idx, false);
            Ok(())
        }
    }
}

struct FreeBlockBitmap {
    map: BitArray<[u8; FREE_BLK_BMAP_SIZE_BYTES], Lsb0>,
}

impl Default for FreeBlockBitmap {
    fn default() -> Self {
        let mut map = BitArray::default();
        map[0..(RESERVED_DATA_BLKS as usize)].fill(true);
        Self { map }
    }
}

impl FreeObjectBitmap<FREE_BLK_BMAP_SIZE_BYTES> for FreeBlockBitmap {
    const RESERVED: usize = RESERVED_DATA_BLKS as usize; 
    const MAX: usize = NUM_DATA_BLKS as usize;
    fn map(&mut self) -> &mut BitArray<[u8; FREE_BLK_BMAP_SIZE_BYTES], Lsb0> {
        &mut self.map
    }
}

#[derive(Clone, Copy)]
struct Inode {
    ino_id: u64,     // inode number
    size: u64,       // file size
    blocks: u64,     // num blocks allocated
    mtime_secs: i64, // Easier to save to disk than SystemTime. Ignored the atime and ctime for now.
    kind: FileType,
    perm: u16,
    direct_blks: [u32; NUM_INO_DIRECT_PTR],
    indirect_blk: u32,
    dbl_indirect_blk: u32,
    tri_indirect_blk: u32,
}

fn secs_from_unix_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Inode {
    fn new(ino_id: u64, kind: FileType, perm: u16) -> Self {
        Self {
            ino_id,
            size: 0,
            blocks: 0,
            mtime_secs: secs_from_unix_epoch(),
            kind,
            perm,
            direct_blks: [INVALID_PTR; NUM_INO_DIRECT_PTR],
            indirect_blk: INVALID_PTR,
            dbl_indirect_blk: INVALID_PTR,
            tri_indirect_blk: INVALID_PTR,
        }
    }
}

struct FreeInodeBitmap {
    map: BitArray<[u8; FREE_INODE_BMAP_SIZE_BYTES], Lsb0>,
}

impl Default for FreeInodeBitmap {
    fn default() -> Self {
        let mut map = BitArray::default();
        map[0..(RESERVED_INODES as usize)].fill(true);
        Self { map }
    }
}

impl FreeObjectBitmap<FREE_INODE_BMAP_SIZE_BYTES> for FreeInodeBitmap {
    const RESERVED: usize = RESERVED_INODES as usize;
    const MAX: usize = MAX_NUM_INODES as usize;
    fn map(&mut self) -> &mut BitArray<[u8; FREE_INODE_BMAP_SIZE_BYTES], Lsb0> {
        &mut self.map
    }
}

struct FSState {
    metadata: FSMetadata,
    inode_bitmap: FreeInodeBitmap,
    inodes: [Option<Inode>; MAX_NUM_INODES as usize],
    free_blk_bitmap: FreeBlockBitmap,
    blks: [Option<Block>; NUM_DATA_BLKS as usize],
}

#[derive(Debug)]
enum InodeError {
    NoFreeInodesOnAlloc,
    InodeNotFound,
    InvalidInoId
}

impl FSState {
    fn alloc_inode(&mut self, kind: FileType, perm: u16) -> Result<u64, InodeError> {
        let ino_idx: usize = match self.inode_bitmap.find_first_free() {
            Some(idx) => idx,
            None => {
                error!("alloc_inode failed: no free inodes");
                return Err(InodeError::NoFreeInodesOnAlloc);
            }
        };

        // TODO: add root inode logic 
        // TODO: Block first two blocks.
        // TODO: Reserved ones  
        // mark inode allocated in bitmap
        self.inode_bitmap.map.set(ino_idx, true);

        // The true index is 0, but we use 0 as an invalid ptr so always 
        // increment when storing ptrs 
        let ino_id: u64 = (ino_idx + 1) as u64; 
        self.inodes[ino_idx] = Some(Inode::new(ino_id, kind, perm));

        Ok(ino_id)
    }

    fn fnid_inode_by_id(&self, ino_id: u64) -> Result<&Inode, InodeError> {
        if ino_id <= 0 {
            return Err(InodeError::InvalidInoId);
        }
        let ino_idx = (ino_id - 1) as usize;
        if ino_idx >= (MAX_NUM_INODES as usize) {
            return Err(InodeError::InodeNotFound);
        }

        match self.inodes[ino_idx].as_ref() {
            Some(inode) => Ok(inode),
            None => Err(InodeError::InodeNotFound),
        } 
    }
} 

// we have to implement Default ourselves here
// because the Default trait is not implemented for static arrays
// above a certain size
impl Default for FSState {
    fn default() -> Self {
        let metadata = FSMetadata::default();
        let inode_bitmap = FreeInodeBitmap::default();
        let inodes = [None; MAX_NUM_INODES as usize];
        let free_blk_bitmap = FreeBlockBitmap::default();
        let blks = [None; NUM_DATA_BLKS as usize];

        Self {
            metadata,
            inode_bitmap,
            inodes,
            free_blk_bitmap,
            blks,
        }
    }
}

fn main() {
    env_logger::init();
    let mountpoint = env::args_os().nth(1).unwrap();
    fuser::mount2(NullFS, mountpoint, &[MountOption::AutoUnmount]).unwrap();
}
