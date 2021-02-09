use crate::FarcFile;
use crc::crc32;
use std::collections::HashMap;

fn string_to_utf16(to_transform: &str) -> Vec<u8> {
    to_transform
        .encode_utf16()
        .map(|chara| chara.to_le_bytes().to_vec())
        .flatten()
        .collect()
}

#[derive(Debug, Default)]
pub struct FileNameIndex {
    file_data: Vec<FarcFile>,
    pub name_crc32: HashMap<u32, usize>,
    pub name_string: HashMap<String, usize>,
}

impl FileNameIndex {
    pub fn add_file_with_hash(&mut self, hash: u32, offset: u32, length: u32) {
        let file_id = self.file_data.len();
        self.file_data.push(FarcFile::new(offset, length));
        self.name_crc32.insert(hash, file_id);
    }

    pub fn add_file_with_name(&mut self, name: String, offset: u32, lenght: u32) {
        let file_id = self.file_data.len();
        self.file_data.push(FarcFile::new(offset, lenght));
        self.name_string.insert(name, file_id);
    }

    pub fn check_file_name(&mut self, name: &str) -> bool {
        let name_encoded_utf16 = string_to_utf16(name);
        let hash = crc32::checksum_ieee(&name_encoded_utf16);
        if self.name_crc32.contains_key(&hash) {
            trace!("found a corresponding hash: {} <-> {}", hash, name);
            let file_id = self.name_crc32.remove(&hash).unwrap(); //should always work
            if self.name_string.insert(name.to_string(), file_id).is_some() {
                panic!("hash mismach !!! Maybe try to rename the file {:?}", name);
            };
            true
        } else {
            if log_enabled!(log::Level::Trace) && !self.name_string.contains_key(name) {
                trace!("hash not found in the file name: {} for {}", hash, name);
            }
            false
        }
    }

    pub fn get_named_file_data(&self, name: &str) -> Option<FarcFile> {
        self.file_data
            .get(match self.name_string.get(name) {
                Some(value) => *value,
                None => match self
                    .name_crc32
                    .get(&crc32::checksum_ieee(&string_to_utf16(name)))
                {
                    Some(value) => *value,
                    None => return None,
                },
            })
            .cloned()
    }

    pub fn get_unnamed_file_data(&self, hash: u32) -> Option<FarcFile> {
        self.file_data
            .get(match self.name_crc32.get(&hash) {
                Some(value) => *value,
                None => return None,
            })
            .cloned()
    }
}
