use bitvec::prelude::*;
use fuser::{FileType, Filesystem, MountOption};
use log::{debug, error, info, log_enabled, Level};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, error};

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

#[derive(Debug)]
enum FSMetadataError {
    InoCountExceedingMax,
    InoCountBelowReserved,
}
impl FSMetadata {
    fn dec_free_ino_count(&mut self) -> Result<(), FSMetadataError> {
        if self.free_ino_count < 0 {
            error!(
                "Attempted to decrease the inode count below reserved: {}",
                { RESERVED_INODES }
            );
            Err(FSMetadataError::InoCountBelowReserved)
        } else {
            self.free_ino_count -= 1;
            self.mtime = secs_from_unix_epoch() as u64;
            Ok(())
        }
    }

    fn inc_free_ino_count(&mut self) -> Result<(), FSMetadataError> {
        if self.free_ino_count >= (MAX_NUM_INODES - RESERVED_INODES) {
            error!(
                "Attempted to increase the inode count above max: {}",
                MAX_NUM_INODES - RESERVED_INODES
            );
            Err(FSMetadataError::InoCountExceedingMax)
        } else {
            self.free_ino_count += 1;
            self.mtime = secs_from_unix_epoch() as u64;
            Ok(())
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
    AlreadyFree,
}

trait FreeObjectBitmap<const N: usize> {
    const RESERVED: usize;
    const MAX: usize;

    fn map(&mut self) -> &mut BitArray<[u8; N], Lsb0>;

    fn find_first_free(&mut self) -> Option<usize> {
        for idx in Self::RESERVED..Self::MAX {
            if !self.map()[idx] {
                return Some(idx);
            }
        }
        None
    }

    fn set_alloc(&mut self, idx: usize) -> Result<(), BitMapError> {
        if idx < Self::RESERVED || idx >= Self::MAX {
            error!("Tried to acces restricted index: {idx}");
            return Err(BitMapError::RestrictedEntry);
        }
        if self.map()[idx] == true {
            error!("The index is already alloced, no change");
            return Err(BitMapError::AlreadyAlloced);
        } else {
            self.map().set(idx, true);
            Ok(())
        }
    }

    fn set_free(&mut self, idx: usize) -> Result<(), BitMapError> {
        if idx < Self::RESERVED || idx >= Self::MAX {
            error!("Tried to acces restricted index: {idx}");
            return Err(BitMapError::RestrictedEntry);
        }
        if self.map()[idx] == false {
            error!("The index is already free, no change");
            return Err(BitMapError::AlreadyFree);
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

#[derive(Clone, Copy, PartialEq, Debug)]
// Because of Copy, re-assignment of variable is copied; ownership is not transferred.
// Use references here.
struct Inode {
    ino_id: u32,     // inode number
    size: u64,       // file size
    blocks: u32,     // num blocks allocated
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
    fn new(ino_id: u32, kind: FileType, perm: u16) -> Self {
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

    fn update_mtime(&mut self) {
        self.mtime_secs = secs_from_unix_epoch();
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
    inodes: Box<[Option<Inode>]>,
    blk_bitmap: FreeBlockBitmap,
    blks: Box<[Option<Block>]>,
}

#[derive(Debug)]
enum InodeError {
    NoFreeInodesOnAlloc,
    InodeNotFound,
    InvalidInoId,
    BitmapError(BitMapError),
}

impl Default for FSState {
    fn default() -> Self {
        let metadata = FSMetadata::default();
        let inode_bitmap = FreeInodeBitmap::default();
        let inodes = vec![None; MAX_NUM_INODES as usize].into_boxed_slice();
        let blk_bitmap = FreeBlockBitmap::default();
        let blks = vec![None; NUM_DATA_BLKS as usize].into_boxed_slice();

        Self {
            metadata,
            inode_bitmap,
            inodes,
            blk_bitmap,
            blks,
        }
    }
}

impl FSState {
    fn new(
        metadata: FSMetadata,
        inode_bitmap: FreeInodeBitmap,
        inodes: Box<[Option<Inode>; MAX_NUM_INODES as usize]>,
        blk_bitmap: FreeBlockBitmap,
        blks: Box<[Option<Block>; NUM_DATA_BLKS as usize]>,
    ) -> Self {
        Self {
            metadata,
            inode_bitmap,
            inodes,
            blk_bitmap,
            blks,
        }
    }

    fn alloc_inode(&mut self, kind: FileType, perm: u16) -> Result<u32, InodeError> {
        let idx = self
            .inode_bitmap
            .find_first_free()
            .ok_or(InodeError::NoFreeInodesOnAlloc)?;

        self.inode_bitmap
            .set_alloc(idx)
            .map_err(|_| InodeError::NoFreeInodesOnAlloc)?;

        self.metadata
            .dec_free_ino_count()
            .map_err(|_| InodeError::NoFreeInodesOnAlloc)?;

        self.inodes[idx] = Some(Inode::new(idx as u32, kind, perm));
        Ok(idx as u32)
    }

    fn free_inode(&mut self, ino_id: u32) -> Result<(), InodeError> {
        let idx = ino_id as usize;

        self.inode_bitmap.set_free(idx).map_err(|err| match err {
            BitMapError::RestrictedEntry => InodeError::InvalidInoId,
            BitMapError::AlreadyFree => InodeError::BitmapError(BitMapError::AlreadyFree),
            BitMapError::AlreadyAlloced => InodeError::BitmapError(BitMapError::AlreadyAlloced),
        })?;

        self.metadata
            .inc_free_ino_count()
            .map_err(|_| InodeError::InvalidInoId)?;

        self.inodes[idx] = None;
        Ok(())
    }
}

fn main() {
    env_logger::init();
    let mountpoint = env::args_os().nth(1).unwrap();
    fuser::mount2(NullFS, mountpoint, &[MountOption::AutoUnmount]).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test find_first_free
    #[test]
    fn test_find_first_free_returns_first_unreserved_index() {
        let mut bitmap = FreeInodeBitmap::default();
        assert_eq!(bitmap.find_first_free(), Some(RESERVED_INODES as usize));
    }

    #[test]
    fn test_find_first_free_skips_allocated_indices() {
        let mut bitmap = FreeInodeBitmap::default();
        bitmap.map.set(2, true);
        assert_eq!(bitmap.find_first_free(), Some(3));
    }

    #[test]
    fn test_find_first_free_returns_none_when_full() {
        let mut bitmap = FreeInodeBitmap::default();
        bitmap.map.fill(true);
        assert_eq!(bitmap.find_first_free(), None);
    }

    // Test set_alloc
    #[test]
    fn test_set_alloc_succeeds_for_valid_free_index() {
        let mut bitmap = FreeInodeBitmap::default();
        let idx = RESERVED_INODES as usize;
        assert!(bitmap.set_alloc(idx).is_ok());
        assert_eq!(bitmap.map[idx], true);
    }

    #[test]
    fn test_set_alloc_fails_for_reserved_index() {
        let mut bitmap = FreeInodeBitmap::default();
        let result = bitmap.set_alloc(0);
        assert!(matches!(result, Err(BitMapError::RestrictedEntry)));
    }

    #[test]
    fn test_set_alloc_fails_for_index_beyond_max() {
        let mut bitmap = FreeInodeBitmap::default();
        let result = bitmap.set_alloc(MAX_NUM_INODES as usize + 1);
        assert!(matches!(result, Err(BitMapError::RestrictedEntry)));
    }

    #[test]
    fn test_set_alloc_fails_for_already_allocated_index() {
        let mut bitmap = FreeInodeBitmap::default();
        let idx = RESERVED_INODES as usize;
        bitmap.map.set(idx, true);
        let result = bitmap.set_alloc(idx);
        assert!(matches!(result, Err(BitMapError::AlreadyAlloced)));
    }

    // Test set_free
    #[test]
    fn test_set_free_succeeds_for_valid_allocated_index() {
        let mut bitmap = FreeInodeBitmap::default();
        let idx = RESERVED_INODES as usize;
        bitmap.map.set(idx, true); // First allocate it
        assert!(bitmap.set_free(idx).is_ok());
        assert_eq!(bitmap.map[idx], false);
    }

    #[test]
    fn test_set_free_fails_for_reserved_index() {
        let mut bitmap = FreeInodeBitmap::default();
        let result = bitmap.set_free(0);
        assert!(matches!(result, Err(BitMapError::RestrictedEntry)));
        assert_eq!(bitmap.map[0], true)
    }

    #[test]
    fn test_set_free_fails_for_index_beyond_max() {
        let mut bitmap = FreeInodeBitmap::default();
        let result = bitmap.set_free(MAX_NUM_INODES as usize + 1);
        assert!(matches!(result, Err(BitMapError::RestrictedEntry)));
    }

    #[test]
    fn test_set_free_fails_for_already_free_index() {
        let mut bitmap = FreeInodeBitmap::default();
        let idx = RESERVED_INODES as usize;
        let result = bitmap.set_free(idx);
        assert!(matches!(result, Err(BitMapError::AlreadyFree)));
    }

    // Test with FreeBlockBitmap to ensure trait works for both implementations
    #[test]
    fn test_free_block_bitmap_find_first_free() {
        let mut bitmap = FreeBlockBitmap::default();
        assert_eq!(bitmap.find_first_free(), Some(RESERVED_DATA_BLKS as usize));
    }

    #[test]
    fn test_free_block_bitmap_set_alloc_and_free() {
        let mut bitmap = FreeBlockBitmap::default();
        let idx = RESERVED_DATA_BLKS as usize;

        // Allocate
        assert!(bitmap.set_alloc(idx).is_ok());
        assert_eq!(bitmap.map[idx], true);

        // Free
        assert!(bitmap.set_free(idx).is_ok());
        assert_eq!(bitmap.map[idx], false);
    }

    #[test]
    fn test_free_block_bitmap_max() {
        let mut bitmap = FreeBlockBitmap::default();
        let idx = NUM_DATA_BLKS as usize;
        let idx2 = 4 as usize;
        assert!(bitmap.set_alloc(idx2).is_ok());
        assert_eq!(bitmap.map[idx2], true);

        let result = bitmap.set_alloc(idx);
        assert!(matches!(result, Err(BitMapError::RestrictedEntry)));
    }

    #[test]
    fn test_basic_innode_alloc_and_free_no_errors() {
        let fsstate = &mut FSState::default();
        let result = fsstate.alloc_inode(FileType::RegularFile, 0);
        let expected_idx = RESERVED_INODES;

        assert!(result.is_ok());
        let ino_id = result.unwrap();
        assert_eq!(ino_id, expected_idx);

        let free_res = fsstate.free_inode(ino_id);
        assert!(free_res.is_ok());

        assert_eq!(fsstate.inodes[ino_id as usize], None);
    }

    #[test]
    fn test_free_inode_once_succeeds_twice_fails() {
        let fsstate = &mut FSState::default();
        let ino_id = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();

        // First free should succeed
        assert!(fsstate.free_inode(ino_id).is_ok());

        // Second free should fail
        let result = fsstate.free_inode(ino_id);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            InodeError::BitmapError(BitMapError::AlreadyFree)
        ));
    }

    #[test]
    fn test_sequential_allocation_indices() {
        let fsstate = &mut FSState::default();

        let ino1 = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();
        let ino2 = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();
        let ino3 = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();

        assert_eq!(ino1, RESERVED_INODES);
        assert_eq!(ino2, RESERVED_INODES + 1);
        assert_eq!(ino3, RESERVED_INODES + 2);
    }

    #[test]
    fn test_free_both_and_reallocate_with_bitmap_verification() {
        let fsstate = &mut FSState::default();

        // Allocate two inodes
        let ino1 = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();
        let ino2 = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();

        // Verify bitmap is set
        assert_eq!(fsstate.inode_bitmap.map[ino1 as usize], true);
        assert_eq!(fsstate.inode_bitmap.map[ino2 as usize], true);

        // Free both
        fsstate.free_inode(ino1).unwrap();
        fsstate.free_inode(ino2).unwrap();

        // Verify bitmap is cleared
        assert_eq!(fsstate.inode_bitmap.map[ino1 as usize], false);
        assert_eq!(fsstate.inode_bitmap.map[ino2 as usize], false);
        assert_eq!(fsstate.inodes[ino1 as usize], None);
        assert_eq!(fsstate.inodes[ino2 as usize], None);

        // Reallocate and verify bitmap is set again
        let ino_new = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();
        assert_eq!(ino_new, RESERVED_INODES);
        assert_eq!(fsstate.inode_bitmap.map[ino_new as usize], true);
        assert!(fsstate.inodes[ino_new as usize].is_some());
    }

    #[test]
    fn test_allocate_all_inodes_to_max() {
        let fsstate = &mut FSState::default();
        let mut allocated_inodes = Vec::new();

        // Allocate all available inodes (MAX - RESERVED)
        for _ in 0..(MAX_NUM_INODES - RESERVED_INODES) {
            let ino = fsstate.alloc_inode(FileType::RegularFile, 0);
            assert!(ino.is_ok());
            allocated_inodes.push(ino.unwrap());
        }

        // Verify we allocated the expected number
        assert_eq!(
            allocated_inodes.len(),
            (MAX_NUM_INODES - RESERVED_INODES) as usize
        );

        assert_eq!(fsstate.metadata.ino_count, MAX_NUM_INODES);
        assert_eq!(fsstate.metadata.free_ino_count, 0);

        // Try to allocate one more - should fail
        let result = fsstate.alloc_inode(FileType::RegularFile, 0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            InodeError::NoFreeInodesOnAlloc
        ));
    }

    #[test]
    fn test_free_reserved_inode_0_fails() {
        let fsstate = &mut FSState::default();

        let result = fsstate.free_inode(0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), InodeError::InvalidInoId));
    }

    #[test]
    fn test_free_reserved_inode_1_fails() {
        let fsstate = &mut FSState::default();

        let result = fsstate.free_inode(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), InodeError::InvalidInoId));
    }

    #[test]
    fn test_metadata_counts_during_alloc_and_free() {
        let fsstate = &mut FSState::default();

        let initial_free_count = fsstate.metadata.free_ino_count;

        // Allocate
        let ino = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();
        assert_eq!(fsstate.metadata.ino_count, MAX_NUM_INODES);
        assert_eq!(fsstate.metadata.free_ino_count, initial_free_count - 1);

        // Free
        fsstate.free_inode(ino).unwrap();
        assert_eq!(fsstate.metadata.ino_count, MAX_NUM_INODES);
        assert_eq!(fsstate.metadata.free_ino_count, initial_free_count);
    }

    #[test]
    fn test_alloc_free_alloc_reuses_same_index() {
        let fsstate = &mut FSState::default();

        let ino1 = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();
        fsstate.free_inode(ino1).unwrap();
        let ino2 = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();

        // Should reuse the same index
        assert_eq!(ino1, ino2);
    }

    #[test]
    fn test_inode_properties_set_correctly() {
        let fsstate = &mut FSState::default();

        let ino_id = fsstate.alloc_inode(FileType::Directory, 0o755).unwrap();
        let inode = fsstate.inodes[ino_id as usize].as_ref().unwrap();

        assert_eq!(inode.ino_id, ino_id);
        assert_eq!(inode.kind, FileType::Directory);
        assert_eq!(inode.perm, 0o755);
        assert_eq!(inode.size, 0);
        assert_eq!(inode.blocks, 0);
        assert!(inode.mtime_secs > 0);
    }

    #[test]
    fn test_free_middle_inode_and_reallocate() {
        let fsstate = &mut FSState::default();

        let ino1 = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();
        let ino2 = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();
        let ino3 = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();

        // Free middle inode
        fsstate.free_inode(ino2).unwrap();

        // Allocate again - should get ino2 back (first free slot)
        let ino_new = fsstate.alloc_inode(FileType::RegularFile, 0).unwrap();
        assert_eq!(ino_new, ino2);

        // ino1 and ino3 should still be allocated
        assert!(fsstate.inodes[ino1 as usize].is_some());
        assert!(fsstate.inodes[ino3 as usize].is_some());
    }
}
