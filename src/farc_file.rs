#[derive(Debug, Clone)]
pub struct FarcFile {
    pub start: u32,
    pub length: u32,
    pub crc32: u32,
    pub name: Option<String>,
}

impl FarcFile {
    pub fn new(start: u32, length: u32, crc32: u32, name: Option<String>) -> Self {
        Self {
            start,
            length,
            crc32,
            name,
        }
    }
}
