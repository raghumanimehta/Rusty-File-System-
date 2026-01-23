use crate::fs::bitmap::BitMapError;
use crate::fs::metadata::secs_from_unix_epoch;
use fuser::FileType;
pub const ROOT_INO: u32 = 1;
pub const ROOT_INO_PERM: u16 = 0o755;

pub const NUM_INO_DIRECT_PTR: usize = 12;
pub const INVALID_PTR: u32 = 0;

#[derive(Clone, Copy, PartialEq, Debug)]
// Because of Copy, re-assignment of variable is copied; ownership is not transferred.
// Use references here.
pub struct Inode {
    pub ino_id: u32,     // inode number
    pub size: u64,       // file size
    pub blocks: u32,     // num blocks allocated
    pub mtime_secs: i64, // Easier to save to disk than SystemTime. Ignored the atime and ctime for now.
    pub kind: FileType,
    pub perm: u16,
    pub direct_blks: [u32; NUM_INO_DIRECT_PTR],
    pub indirect_blk: u32,
    pub dbl_indirect_blk: u32,
    pub tri_indirect_blk: u32,
}

#[derive(Debug)]
pub enum InodeError {
    NoFreeInodesOnAlloc,
    InodeNotFound,
    InvalidInoId,
    BitmapError(BitMapError),
}

impl Inode {
    pub fn new(ino_id: u32, kind: FileType, perm: u16) -> Self {
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

    pub fn update_mtime(&mut self) {
        self.mtime_secs = secs_from_unix_epoch();
    }
}

pub fn create_root_ino() -> Inode {
    return Inode::new(ROOT_INO, FileType::Directory, ROOT_INO_PERM);
}
