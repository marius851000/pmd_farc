use crc::crc32;
use io_partition::PartitionMutex;
use pmd_sir0::{Sir0, Sir0Error};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::string::FromUtf16Error;
use std::sync::{Arc, Mutex};

/// An error that ``Farc`` can return
#[derive(Debug)]
pub enum FarcError {
    /// An error happened while performing an io
    IOerror(io::Error),
    /// The Type of the Farc is not reconized
    InvalidType(u32),
    /// An error happened while creating a ``Partition``
    PartitionCreationError(io::Error),
    /// An error happened while creating a ``Sir0``
    CreateSir0Error(Sir0Error),
    /// The Fat5 type is not reconized
    UnsuportedFat5Type(u32),
    /// The Mutex containing the file was poisoned
    Poisoned,
    /// A file with a name was not found
    NamedFileNotFound(String),
    /// A file with a hash was not found
    HashedFileNotFound(u32),
    /// An error happened while creating an utf16 string
    FromUtf16Error(FromUtf16Error),
}

impl Error for FarcError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IOerror(err) | Self::PartitionCreationError(err) => Some(err),
            Self::CreateSir0Error(err) => Some(err),
            Self::FromUtf16Error(err) => Some(err),
            Self::InvalidType(_)
            | Self::UnsuportedFat5Type(_)
            | Self::Poisoned
            | Self::NamedFileNotFound(_)
            | Self::HashedFileNotFound(_) => None,
        }
    }
}

impl Display for FarcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IOerror(_) => write!(f, "An error occured while performing an IO operation"),
            Self::InvalidType(id) => write!(
                f,
                "The type of the file is not reconized: found type {}",
                id
            ),
            Self::PartitionCreationError(_) => write!(
                f,
                "An error happened while creating a partition of the file"
            ),
            Self::CreateSir0Error(_) => write!(f, "An error happened while creating a Sir0 file"),
            Self::UnsuportedFat5Type(id) => {
                write!(f, "The fat5 type is not supported: found {}", id)
            }
            Self::Poisoned => write!(f, "The mutex guarding the access to the file is poisoned"),
            Self::NamedFileNotFound(name) => {
                write!(f, "The file with name \"{}\" does not exist", name)
            }
            Self::HashedFileNotFound(hash) => {
                write!(f, "The file with the hash \"{}\" does not exist", hash)
            }
            Self::FromUtf16Error(_) => {
                write!(f, "An error happened while parsing an utf-16 string")
            }
        }
    }
}

impl From<FromUtf16Error> for FarcError {
    fn from(err: FromUtf16Error) -> Self {
        Self::FromUtf16Error(err)
    }
}

impl From<io::Error> for FarcError {
    fn from(err: io::Error) -> Self {
        Self::IOerror(err)
    }
}

fn read_u32_le<T: Read>(file: &mut T) -> Result<u32, FarcError> {
    let mut buffer = [0; 4];
    file.read_exact(&mut buffer)?;
    Ok(u32::from_le_bytes(buffer))
}

fn read_u16_le<T: Read>(file: &mut T) -> Result<u16, FarcError> {
    let mut buffer = [0; 2];
    file.read_exact(&mut buffer)?;
    Ok(u16::from_le_bytes(buffer))
}

fn read_null_terminated_utf16_string<T: Read>(file: &mut T) -> Result<String, FarcError> {
    let mut buffer: Vec<u16> = Vec::new();
    loop {
        let chara = read_u16_le(file)?;
        if chara == 0 {
            break;
        };
        buffer.push(chara);
    }
    Ok(String::from_utf16(&buffer)?)
}

#[derive(Debug, Clone)]
struct FarcFile {
    start: u32,
    length: u32,
}

impl FarcFile {
    fn new(start: u32, length: u32) -> Self {
        Self { start, length }
    }
}

fn string_to_utf16(to_transform: &str) -> Vec<u8> {
    to_transform
        .encode_utf16()
        .map(|chara| chara.to_le_bytes().to_vec())
        .flatten()
        .collect()
}

#[derive(Debug, Default)]
struct FileNameIndex {
    file_data: Vec<FarcFile>,
    name_crc32: HashMap<u32, usize>,
    name_string: HashMap<String, usize>,
}

impl FileNameIndex {
    fn add_file_with_hash(&mut self, hash: u32, offset: u32, length: u32) {
        let file_id = self.file_data.len();
        self.file_data.push(FarcFile::new(offset, length));
        self.name_crc32.insert(hash, file_id);
    }

    fn add_file_with_name(&mut self, name: String, offset: u32, lenght: u32) {
        let file_id = self.file_data.len();
        self.file_data.push(FarcFile::new(offset, lenght));
        self.name_string.insert(name, file_id);
    }

    fn check_file_name(&mut self, name: &str) -> bool {
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

    fn get_named_file_data(&self, name: &str) -> Option<FarcFile> {
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

    fn get_unnamed_file_data(&self, hash: u32) -> Option<FarcFile> {
        self.file_data
            .get(match self.name_crc32.get(&hash) {
                Some(value) => *value,
                None => return None,
            })
            .cloned()
    }
}

#[derive(Debug)]
/// A parser for a file in the farc format (see the crate root documentation for more information)
pub struct Farc<F: Read + Seek> {
    file: Arc<Mutex<F>>,
    index: FileNameIndex,
}

impl<F: Read + Seek> Farc<F> {
    /// Create and parse a new ``Farc`` object, with the specified input file
    pub fn new(file: F) -> Result<Self, FarcError> {
        let file = Arc::new(Mutex::new(file));
        let sir0_type;
        let sir0_offset;
        let sir0_lenght;
        let all_data_offset;
        {
            let mut file = file.lock().unwrap(); // shouldn't panic
            file.seek(SeekFrom::Start(0x20))?;
            //0x20
            sir0_type = read_u32_le(&mut *file)?;
            if sir0_type != 4 && sir0_type != 5 {
                return Err(FarcError::InvalidType(sir0_type));
            }
            //0x24
            sir0_offset = read_u32_le(&mut *file)?;
            //0x28
            sir0_lenght = read_u32_le(&mut *file)?;
            //0x2C
            all_data_offset = read_u32_le(&mut *file)?;
        }

        let sir0_partition =
            io_partition::PartitionMutex::new(file.clone(), sir0_offset as u64, sir0_lenght as u64)
                .map_err(FarcError::PartitionCreationError)?;
        let mut sir0 = Sir0::new(sir0_partition).map_err(FarcError::CreateSir0Error)?;

        let h = sir0.get_header();
        let sir0_data_offset = u32::from_le_bytes([h[0], h[1], h[2], h[3]]);
        let file_count = u32::from_le_bytes([h[4], h[5], h[6], h[7]]);
        let sir0_fat5_type = u32::from_le_bytes([h[8], h[9], h[10], h[11]]);

        let entry_lenght = match sir0_fat5_type {
            0 => 12, //TODO: difference with the evandixon implementation
            1 => 12,
            x => return Err(FarcError::UnsuportedFat5Type(x)),
        };

        let mut index = FileNameIndex::default();
        let mut sir0_file = sir0.get_file();
        for file_index in 0..(file_count) {
            sir0_file.seek(SeekFrom::Start(
                sir0_data_offset as u64 + (file_index * entry_lenght) as u64,
            ))?;
            let filename_offset_or_hash = read_u32_le(&mut sir0_file)?;
            let data_offset = read_u32_le(&mut sir0_file)?;
            let data_length = read_u32_le(&mut sir0_file)?;

            match sir0_fat5_type {
                0 => {
                    sir0_file.seek(SeekFrom::Start(filename_offset_or_hash as u64))?;
                    let name = read_null_terminated_utf16_string(&mut sir0_file)?;
                    index.add_file_with_name(name, all_data_offset + data_offset, data_length);
                }
                1 => index.add_file_with_hash(
                    filename_offset_or_hash,
                    all_data_offset + data_offset,
                    data_length,
                ),
                x => return Err(FarcError::UnsuportedFat5Type(x)),
            };
        }

        Ok(Self { file, index })
    }

    /// return the number of file contained in this ``Farc`` file
    pub fn file_count(&self) -> usize {
        self.file_count_hashed() + self.file_count_named()
    }

    /// return the number of file with an unknown name in this ``Farc`` file
    pub fn file_count_hashed(&self) -> usize {
        self.index.name_crc32.len()
    }

    /// return the number of file with a known name in this ``Farc`` file
    pub fn file_count_named(&self) -> usize {
        self.index.name_string.len()
    }

    /// iter over the known name of file
    pub fn iter_name(&self) -> std::vec::IntoIter<&String> {
        self.index
            .name_string
            .iter()
            .map(|x| x.0)
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// iter over all the hash without an occording known name
    pub fn iter_hash(&self) -> std::vec::IntoIter<u32> {
        self.index
            .name_crc32
            .iter()
            .map(|x| *x.0)
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Return an handle to a file stored in this ``Farc``, from it's name. It will hash the name as necessary.
    pub fn get_named_file(&self, name: &str) -> Result<PartitionMutex<F>, FarcError> {
        let file_data = match self.index.get_named_file_data(name) {
            Some(value) => value,
            None => return Err(FarcError::NamedFileNotFound(name.to_string())),
        };
        self.create_partition_from_data(file_data)
    }

    /// Return an handle to a file with an unknown name. It won't search for file a known name, as opposed to ``get_named_file``
    pub fn get_unnamed_file(&self, hash: u32) -> Result<PartitionMutex<F>, FarcError> {
        let file_data = match self.index.get_unnamed_file_data(hash) {
            Some(value) => value,
            None => return Err(FarcError::HashedFileNotFound(hash)),
        };
        self.create_partition_from_data(file_data)
    }

    fn create_partition_from_data(
        &self,
        file_data: FarcFile,
    ) -> Result<PartitionMutex<F>, FarcError> {
        PartitionMutex::new(
            self.file.clone(),
            file_data.start as u64,
            file_data.length as u64,
        )
        .map_err(FarcError::PartitionCreationError)
    }

    /// Check if the file name correspond to an hash. If it is the case, it replace the hash with name.
    pub fn check_file_name(&mut self, name: &str) -> bool {
        self.index.check_file_name(name)
    }

    /// Call ``check_file_name`` repeteatelly with an iterator
    pub fn check_file_name_iter<T: Iterator<Item = String>>(&mut self, iter: T) {
        for value in iter {
            self.check_file_name(&value);
        }
    }
}
