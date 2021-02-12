use crate::FarcFile;
use crc::crc32;
use std::collections::HashMap;
use thiserror::Error;

fn string_to_utf16(to_transform: &str) -> Vec<u8> {
    to_transform
        .encode_utf16()
        .map(|chara| chara.to_le_bytes().to_vec())
        .flatten()
        .collect()
}

/// Hash a name, first transforming it into utf16, then applying the ieee crc32 checksum
pub fn hash_name(name: &str) -> u32 {
    let name_encoded_utf16 = string_to_utf16(name);
    crc32::checksum_ieee(&name_encoded_utf16)
}

#[derive(Error, Debug)]
/// Any error that may happend due to name conflict
pub enum FileNameError {
    /// two file with the same hash
    #[error("there is already a file with the hash {0} in the farc file.")]
    HashAlreadyPresent(u32),
    /// two file with the same hash, with one known name
    #[error("there is already a file with the hash {0} in the farc file. (one is from the file {1:?}. Maybe you should rename it)")]
    HashAlreadyPresentOne(u32, String),
    /// two file with the same hash, with both name known
    #[error("there is already a file with the hash {0} in the farc file. (one is from the file {1:?}, the second is from {2:?}. Maybe rename one of them)")]
    HashAlreadyPresentTwo(u32, String, String),
    /// two file with the same name
    #[error("there is already a file named {0:?} in the farc file.")]
    NameAlreadyPresent(String),
}

#[derive(Debug, Default)]
/// Represent an index of a FARC file. Each subfile have a known position and lenght related to it's parent file, as well as the hash of the name. The full name may or may not be known for a file.
pub struct FileNameIndex {
    file_data: Vec<FarcFile>,
    file_id_by_crc32: HashMap<u32, usize>,
    file_id_by_string: HashMap<String, usize>,
}

impl FileNameIndex {
    /// Add an entry in this index, with the hash being the crc32 ieee hash of the name encoded as utf16. Return an error if a conflict happen.
    pub fn add_file_with_hash(
        &mut self,
        hash: u32,
        offset: u32,
        lenght: u32,
    ) -> Result<(), FileNameError> {
        let farc_file = FarcFile::new(offset, lenght, hash, None);
        self.add_file(farc_file)
    }

    /// Add an entry to this index, with the name being a standard string. It will internally be converted to the good hash using [`hash_name`].
    /// Return an error if a conflict happen.
    pub fn add_file_with_name(
        &mut self,
        name: String,
        offset: u32,
        lenght: u32,
    ) -> Result<(), FileNameError> {
        let hash = hash_name(&name);
        let farc_file = FarcFile::new(offset, lenght, hash, Some(name));
        self.add_file(farc_file)
    }

    fn add_file(&mut self, farc_file: FarcFile) -> Result<(), FileNameError> {
        let new_farc_id = self.file_data.len();

        if let Some(farc_name) = &farc_file.name {
            if let Some(old_id_by_name) = self
                .file_id_by_string
                .insert(farc_name.to_string(), new_farc_id)
            {
                self.file_id_by_string
                    .insert(farc_name.to_string(), old_id_by_name);
                return Err(FileNameError::NameAlreadyPresent(farc_name.to_string()));
            };
        };

        if let Some(old_id_by_hash) = self
            .file_id_by_crc32
            .insert(farc_file.name_hash, new_farc_id)
        {
            self.file_id_by_crc32
                .insert(farc_file.name_hash, old_id_by_hash);
            if let Some(farc_name) = &farc_file.name {
                self.file_id_by_string.remove(farc_name);
            };
            return Err(if let Some(name_first) = farc_file.name.clone() {
                if let Some(name_second) = self.file_data[old_id_by_hash].name.clone() {
                    FileNameError::HashAlreadyPresentTwo(
                        farc_file.name_hash,
                        name_first,
                        name_second,
                    )
                } else {
                    FileNameError::HashAlreadyPresentOne(farc_file.name_hash, name_first)
                }
            } else {
                if let Some(name_second) = self.file_data[old_id_by_hash].name.clone() {
                    FileNameError::HashAlreadyPresentOne(farc_file.name_hash, name_second)
                } else {
                    FileNameError::HashAlreadyPresent(farc_file.name_hash)
                }
            });
        }

        self.file_data.push(farc_file);
        Ok(())
    }

    /// If a file is found in the index that have a file name hash that correspond to the given name.
    /// If it does, return true, and save this name. otherwise, return false.
    ///
    /// If there is a conflict found, do nothing and return false
    pub fn check_file_name(&mut self, name: &str) -> bool {
        let hash = hash_name(name);
        if let Some(id) = self.file_id_by_crc32.get(&hash) {
            let file = &mut self.file_data[*id];
            if file.name.is_none() {
                file.name = Some(name.to_string());
                self.file_id_by_string.insert(name.to_string(), *id);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Return the file with the given name (the hash of the name is also tested, but not saved).
    /// If there is a conflict with the hash value, None is returned.
    pub fn get_file_by_name(&self, name: &str) -> Option<&FarcFile> {
        if let Some(direct) = self.file_id_by_string.get(name) {
            Some(&self.file_data[*direct])
        } else {
            let hash = hash_name(name);
            if let Some(file_id) = self.file_id_by_crc32.get(&hash) {
                let file = &self.file_data[*file_id];
                if file.name.is_some() {
                    None
                } else {
                    Some(file)
                }
            } else {
                None
            }
        }
    }

    /// Return the file with the conresponding file name hash.
    pub fn get_file_by_hash(&self, hash: u32) -> Option<&FarcFile> {
        if let Some(id) = self.file_id_by_crc32.get(&hash) {
            Some(&self.file_data[*id])
        } else {
            None
        }
    }

    /// return the total number of registered file in this index.
    pub fn len(&self) -> usize {
        self.file_data.len()
    }

    /// iterate over all the file entry, sorted by addition order.
    pub fn iter(&self) -> impl Iterator<Item = &FarcFile> {
        self.file_data.iter()
    }
}
