struct DirEntry {
    pub ino_id: u32,
    pub name: String,
}

impl DirEntry {
    pub fn new(ino_id: u32, name: String) -> Self {
        Self { ino_id, name }
    }
}


