use bitvec::prelude::*;
use log::error;
use crate::fs::metadata::{BLK_SIZE_BYTES, NUM_DATA_BLKS, RESERVED_DATA_BLKS, MAX_NUM_INODES, RESERVED_INODES};


pub const FREE_BLK_BMAP_SIZE_BYTES: usize = (NUM_DATA_BLKS as usize) / (BLK_SIZE_BYTES as usize);
pub const FREE_INODE_BMAP_SIZE_BYTES: usize = (MAX_NUM_INODES as usize + 7) / 8;

#[derive(Debug)]
pub enum BitMapError {
    RestrictedEntry,
    AlreadyAlloced,
    AlreadyFree,
    NoFreeEntriesOnAlloc
}

pub trait FreeObjectBitmap<const N: usize> {
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


pub struct FreeBlockBitmap {
    pub map: BitArray<[u8; FREE_BLK_BMAP_SIZE_BYTES], Lsb0>,  
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

pub struct FreeInodeBitmap {
    pub map: BitArray<[u8; FREE_INODE_BMAP_SIZE_BYTES], Lsb0>,
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
