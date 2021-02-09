#[derive(Debug, Clone)]
pub struct FarcFile {
    pub start: u32,
    pub length: u32,
}

impl FarcFile {
    pub fn new(start: u32, length: u32) -> Self {
        Self { start, length }
    }
}