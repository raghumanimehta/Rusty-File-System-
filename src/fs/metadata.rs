use log::error;
use std::time::{SystemTime, UNIX_EPOCH};

// This is the total capacity of the backing storage for the file system
// this includes the space used for the FSMetadata, free object bitmaps, and file data and metadata
pub const FS_SIZE_BYTES: u64 = 1u64 * (0b1 << 30) as u64; // 1 GB
pub const BLK_SIZE_BYTES: u64 = 4096u64;
// 0 -> FSMetadata, 1->InodeBitmap, 2 -> Freeblock bitmap
pub const RESERVED_DATA_BLKS: u32 = 3;
pub const NUM_DATA_BLKS: u32 = (FS_SIZE_BYTES / BLK_SIZE_BYTES) as u32;

// Inodes
pub const MAX_NUM_INODES: u32 = 10;
pub const RESERVED_INODES: u32 = 2; // 0: null inode, 1: root

pub fn secs_from_unix_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// free inode bitmap can begin right after this struct and inode table can follow immediately after
#[derive(Debug)]
pub struct FSMetadata {
    pub ino_count: u32,
    pub blk_count: u32,
    pub free_blk_count: u32,
    pub free_ino_count: u32,
    pub super_blk_no: u32,
    pub mtime: u64,
    pub wtime: u64,
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
pub enum FSMetadataError {
    InoCountExceedingMax,
    InoCountBelowReserved,
    BlkCountExceedingMax,
    BlkCountBelowReserved,
}

impl FSMetadata {
    pub fn dec_free_ino_count(&mut self) -> Result<(), FSMetadataError> {
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

    pub fn inc_free_ino_count(&mut self) -> Result<(), FSMetadataError> {
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

    pub fn dec_free_blk_count(&mut self) -> Result<(), FSMetadataError> {
        if self.free_blk_count > 0 {
            self.free_blk_count -= 1;
            self.mtime = secs_from_unix_epoch() as u64;
            Ok(())
        } else {
            error!("Attempted to decrease free block count below zero");
            Err(FSMetadataError::BlkCountBelowReserved)
        }
    }
    
    pub fn inc_free_blk_count(&mut self) -> Result<(), FSMetadataError> {
        if self.free_blk_count < (NUM_DATA_BLKS - RESERVED_DATA_BLKS) {
            self.free_blk_count += 1;
            self.mtime = secs_from_unix_epoch() as u64;
            Ok(())
        } else {
            error!("Attempted to increase free block count above max");
            Err(FSMetadataError::BlkCountExceedingMax)
        }
    }
}
