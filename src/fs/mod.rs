pub mod bitmap;
pub mod directory;
pub mod inode;
pub mod metadata;
pub mod state;

use fuser::Filesystem;

pub struct NullFS;
impl Filesystem for NullFS {}
