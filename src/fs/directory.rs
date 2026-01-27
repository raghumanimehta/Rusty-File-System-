use crate::fs::metadata::BLK_SIZE_BYTES;
use log::error;
use std::mem::size_of;

pub const MAX_FILENAME_LEN: usize = 255;
pub const DIR_SIZE_LEN: usize = (BLK_SIZE_BYTES / size_of::<DirEntry>() as u64) as usize;

#[derive(Clone, Copy)]
pub struct DirEntry {
    pub ino_id: u32,
    pub name_len: u8,
    pub name: [u8; MAX_FILENAME_LEN],
}

pub struct Directory {
    dir_entries: Box<[Option<DirEntry>]>,
}

#[derive(Debug)]
pub enum DirectoryError {
    NameTooLong,
    NameEmpty,
    InvalidUtf8,
    NoEmptySlot,
}

impl DirEntry {
    /// Creates a new directory entry with the given inode ID and name
    /// Returns error if name is empty or exceeds MAX_FILENAME_LEN
    pub fn new(ino_id: u32, name: &str) -> Result<Self, DirectoryError> {
        let name_bytes = name.as_bytes();

        if name_bytes.is_empty() {
            error!("Attempted to create DirEntry with empty name");
            return Err(DirectoryError::NameEmpty);
        }

        if name_bytes.len() > MAX_FILENAME_LEN {
            error!(
                "Attempted to create DirEntry with name exceeding {} bytes: {}",
                MAX_FILENAME_LEN,
                name.len()
            );
            return Err(DirectoryError::NameTooLong);
        }

        let mut name_arr = [0u8; MAX_FILENAME_LEN];
        name_arr[..name_bytes.len()].copy_from_slice(name_bytes);

        Ok(Self {
            ino_id,
            name_len: name_bytes.len() as u8,
            name: name_arr,
        })
    }

    /// Returns the name as a string slice
    pub fn name_str(&self) -> Result<&str, DirectoryError> {
        std::str::from_utf8(&self.name[..self.name_len as usize]).map_err(|_| {
            error!("Invalid UTF-8 in directory entry name");
            DirectoryError::InvalidUtf8
        })
    }
}

impl Default for Directory {
    fn default() -> Self {
        Self {
            dir_entries: vec![None; DIR_SIZE_LEN].into_boxed_slice(),
        }
    }
}

pub fn new_dir_with_entries(entries: Box<[Option<DirEntry>]>) -> Result<Directory, DirectoryError> {
    let mut dir = Directory::default();
    let res = dir.set_entries(entries);
    match res {
        Ok(_) => Ok(dir),
        Err(e) => return Err(e),
    }
}

impl Directory {
    /// Sets the directory entries, ensuring they don't exceed the maximum capacity
    pub fn set_entries(&mut self, entries: Box<[Option<DirEntry>]>) -> Result<(), DirectoryError> {
        if entries.len() > DIR_SIZE_LEN {
            error!(
                "Attempted to set entries exceeding max capacity: {} > {}",
                entries.len(),
                DIR_SIZE_LEN
            );
            return Err(DirectoryError::NameTooLong); // Reusing error for now
        }
        self.dir_entries = entries;
        Ok(())
    }
    /*
        fn find_empty_slot(&self) -> Option<usize> {
            for i in 0..self.dir_entries.len() {
                if self.dir_entries[i].is_none() {
                    return i;
                }
            }
        }
    */
    pub fn add_entry(&mut self, enttry: DirEntry) {}
}
