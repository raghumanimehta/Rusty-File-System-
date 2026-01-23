pub mod metadata;
pub mod bitmap;
pub mod inode;
pub mod state;
pub mod directory;

use fuser::Filesystem;

pub struct NullFS;
impl Filesystem for NullFS {}
