pub mod metadata;
pub mod bitmap;
pub mod inode;
pub mod state;

use fuser::Filesystem;

pub struct NullFS;
impl Filesystem for NullFS {}
