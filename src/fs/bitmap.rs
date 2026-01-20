use bitvec::prelude::*;
use log::error;
use crate::fs::metadata::{NUM_DATA_BLKS, RESERVED_DATA_BLKS};

#[derive(Debug)]
pub enum BitMapError {
    RestrictedEntry,
    AlreadyAlloced,
    AlreadyFree,
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
    pub map: BitArray<[u8; 128], Lsb0>,  // FREE_BLK_BMAP_SIZE_BYTES
}

impl Default for FreeBlockBitmap {
    fn default() -> Self {
        let mut map = BitArray::default();
        map[0..(RESERVED_DATA_BLKS as usize)].fill(true);
        Self { map }
    }
}

impl FreeObjectBitmap<128> for FreeBlockBitmap {
    const RESERVED: usize = RESERVED_DATA_BLKS as usize;
    const MAX: usize = NUM_DATA_BLKS as usize;
    fn map(&mut self) -> &mut BitArray<[u8; 128], Lsb0> {
        &mut self.map
    }
}

use crate::fs::metadata::{MAX_NUM_INODES, RESERVED_INODES};

pub struct FreeInodeBitmap {
    pub map: BitArray<[u8; 2], Lsb0>,  // FREE_INODE_BMAP_SIZE_BYTES
}

impl Default for FreeInodeBitmap {
    fn default() -> Self {
        let mut map = BitArray::default();
        map[0..(RESERVED_INODES as usize)].fill(true);
        Self { map }
    }
}

impl FreeObjectBitmap<2> for FreeInodeBitmap {
    const RESERVED: usize = RESERVED_INODES as usize;
    const MAX: usize = MAX_NUM_INODES as usize;
    fn map(&mut self) -> &mut BitArray<[u8; 2], Lsb0> {
        &mut self.map
    }
}
