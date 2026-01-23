use fuser::MountOption;
use std::env;

mod fs;
use fs::NullFS;

fn main() {
    env_logger::init();
    let mountpoint = env::args_os().nth(1).unwrap();
    fuser::mount2(NullFS, mountpoint, &[MountOption::AutoUnmount]).unwrap();
}

#[cfg(test)]
mod tests {
    use crate::fs::bitmap::{BitMapError, FreeBlockBitmap, FreeInodeBitmap, FreeObjectBitmap};
    use crate::fs::metadata::{MAX_NUM_INODES, NUM_DATA_BLKS, RESERVED_DATA_BLKS, RESERVED_INODES};
    use crate::fs::state::{FSState, InodeError};
    use fuser::FileType;

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
