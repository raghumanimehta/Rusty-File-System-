use crate::fs::bitmap::{BitMapError, FreeBlockBitmap, FreeInodeBitmap, FreeObjectBitmap};
use crate::fs::inode::{Inode, InodeError, ROOT_INO};
use crate::fs::metadata::{FSMetadata, MAX_NUM_INODES};
use fuser::FileType;
use log::error;

pub const BLK_SIZE_BYTES: u64 = 4096u64;
pub const NUM_DATA_BLKS: u32 = 262144u32; // 1GB / 4KB

#[derive(Clone, Copy)]
pub struct Block {
    pub data: [u8; BLK_SIZE_BYTES as usize],
}

pub struct FSState {
    pub metadata: FSMetadata,
    pub inode_bitmap: FreeInodeBitmap,
    pub inodes: Box<[Option<Inode>]>,
    pub blk_bitmap: FreeBlockBitmap,
    pub blks: Box<[Option<Block>]>,
}

impl Default for FSState {
    /// Creates a brand new filesystem with initialized root directory
    /// Use this when initializing a new filesystem on the Remote for the first time
    fn default() -> Self {
        let metadata = FSMetadata::default();
        let inode_bitmap = FreeInodeBitmap::default();
        let inodes = vec![None; MAX_NUM_INODES as usize].into_boxed_slice();
        let blk_bitmap = FreeBlockBitmap::default();
        let blks = vec![None; NUM_DATA_BLKS as usize].into_boxed_slice();

        // The bitmap has marked reserved inodes but we have not yet created the root
        // Null inode does not need an inode object allocated to it
        //

        Self {
            metadata,
            inode_bitmap,
            inodes,
            blk_bitmap,
            blks,
        }
    }
}

pub enum FSStateError {
    InodeError(InodeError),
    BitMapError(BitMapError),
}

impl FSState {
    /// Loads an existing filesystem state from Remote
    /// Use this when reconnecting to an already-initialized filesystem
    pub fn new(
        metadata: FSMetadata,
        inode_bitmap: FreeInodeBitmap,
        inodes: Box<[Option<Inode>]>,
        blk_bitmap: FreeBlockBitmap,
        blks: Box<[Option<Block>]>,
    ) -> Self {
        Self {
            metadata,
            inode_bitmap,
            inodes,
            blk_bitmap,
            blks,
        }
    }

    pub fn alloc_inode(&mut self, kind: FileType, perm: u16) -> Result<u32, InodeError> {
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

    pub fn read_ino_alloc(&mut self, read_ino: Inode) -> Result<u32, InodeError> {
        let idx = read_ino.ino_id as usize;
        let is_alloced = self.inode_bitmap.is_alloced(idx);
        match is_alloced {
            Ok(res) => {
                if res {
                    error!("Tried to acces restricted index: {idx}, while reading and allocating an inode");
                    return Err(InodeError::BitmapError(BitMapError::AlreadyAlloced));
                }
                self.inodes[idx] = Some(read_ino);
                Ok(idx as u32)
            }
            Err(_) => Err(InodeError::BitmapError(BitMapError::RestrictedEntry)),
        }
    }
    pub fn free_inode(&mut self, ino_id: u32) -> Result<(), InodeError> {
        let idx = ino_id as usize;

        self.inode_bitmap.set_free(idx).map_err(|err| match err {
            BitMapError::RestrictedEntry => InodeError::InvalidInoId,
            BitMapError::AlreadyFree => InodeError::BitmapError(BitMapError::AlreadyFree),
            BitMapError::AlreadyAlloced => InodeError::BitmapError(BitMapError::AlreadyAlloced),
            BitMapError::NoFreeEntriesOnAlloc => {
                InodeError::BitmapError(BitMapError::NoFreeEntriesOnAlloc)
            }
        })?;

        self.metadata
            .inc_free_ino_count()
            .map_err(|_| InodeError::InvalidInoId)?;

        self.inodes[idx] = None;
        Ok(())
    }

    pub fn alloc_blk(&mut self) -> Result<u32, BitMapError> {
        let idx = self
            .blk_bitmap
            .find_first_free()
            .ok_or(BitMapError::NoFreeEntriesOnAlloc)?;

        self.blk_bitmap.set_alloc(idx)?;

        self.metadata
            .dec_free_blk_count()
            .map_err(|_| BitMapError::RestrictedEntry)?;

        self.blks[idx] = Some(Block {
            data: [0u8; BLK_SIZE_BYTES as usize],
        });

        Ok(idx as u32)
    }

    pub fn free_blk(&mut self, blk_id: u32) -> Result<(), BitMapError> {
        let idx = blk_id as usize;

        self.blk_bitmap.set_free(idx)?;

        self.metadata
            .inc_free_blk_count()
            .map_err(|_| BitMapError::RestrictedEntry)?;

        self.blks[idx] = None;
        Ok(())
    }

    pub fn get_ino_ref(&self, ino_id: u32) -> Result<&Inode, FSStateError> {
        return Ok(self.inodes[ino_id as usize]
            .as_ref()
            .ok_or(FSStateError::InodeError(InodeError::InodeNotFound))?);
    }

    pub fn get_root_ino_ref(&self) -> Result<&Inode, FSStateError> {
        return self.get_ino_ref(ROOT_INO);
    }
}
