use log::error;

pub const MAX_FILENAME_LEN: usize = 255;

#[derive(Clone, Copy)]
pub struct DirEntry {
    pub ino_id: u32,
    pub name_len: u8,
    pub name: [u8; MAX_FILENAME_LEN],
}

#[derive(Debug)]
pub enum DirectoryError {
    NameTooLong,
    NameEmpty,
    InvalidUtf8,
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
