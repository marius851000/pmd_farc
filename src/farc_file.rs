#[derive(Debug, Clone)]
/// Represent a file stored in a farc file
pub struct FarcFile {
    /// The offset since the beggining of the farc file this subfile is present
    pub start: u32,
    /// The lenght of the subfile
    pub length: u32,
    /// the crc32 of the name of this subfile
    pub name_hash: u32,
    /// The name of this subfile
    pub name: Option<String>,
}

impl FarcFile {
    /// Create a new [`FarcFile`] with the given parameter
    pub fn new(start: u32, length: u32, name_hash: u32, name: Option<String>) -> Self {
        Self {
            start,
            length,
            name_hash,
            name,
        }
    }
}
