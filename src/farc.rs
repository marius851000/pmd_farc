use crate::{FarcFile, FileNameError, FileNameIndex};
use binread::{BinRead, BinReaderExt};
use byteorder::{ReadBytesExt, LE};
use io_partition::PartitionMutex;
use pmd_sir0::{Sir0, Sir0Error};
use std::io::{self, Read, Seek, SeekFrom};
use std::string::FromUtf16Error;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// An error that ``Farc`` can return
#[derive(Debug, Error)]
pub enum FarcError {
    /// An error happened while performing an io
    #[error("An error occured while performing an IO operation")]
    IOerror(#[from] io::Error),
    /// An error happened while creating a ``Partition``
    #[error("An error happened while creating a partition of the file")]
    PartitionCreationError(io::Error),
    /// An error happened while creating a ``Sir0``
    #[error("An error happened while creating a Sir0 file")]
    CreateSir0Error(#[from] Sir0Error),
    /// The Fat5 type is not reconized
    #[error("The fat5 type is not supported: found {0}")]
    UnsuportedFat5Type(u32),
    /// The Mutex containing the file was poisoned
    #[error("The mutex guarding the access to the file is poisoned")]
    Poisoned,
    /// A file with a name was not found
    #[error("The file with name \"{0}\" does not exist")]
    NamedFileNotFound(String),
    /// A file with a hash was not found
    #[error("The file with the hash \"{0}\" does not exist")]
    HashedFileNotFound(u32),
    /// An error happened while creating an utf16 string
    #[error("An error happened while parsing an utf-16 string")]
    FromUtf16Error(#[from] FromUtf16Error),
    /// An error caused by parsing the header of the file
    #[error("An error happened while parsing the header of the file")]
    ReadHeaderError(#[source] binread::Error),
    /// The sir0 header isn't long enought
    #[error("The sir0 header isn't long enought. It should be (at least) 12 bytes, but it only have {0} bytes")]
    Sir0HeaderNotLongEnought(usize),
    /// a contained file overflow
    #[error("A contained file is position overflow a u32 integer ({0}+{1})")]
    DataStartOverflow(u32, u32),
    /// a conflict between two file entry
    #[error("A conflict between two file happened")]
    FileNameError(#[from] FileNameError),
}

fn read_null_terminated_utf16_string<T: Read>(file: &mut T) -> Result<String, FarcError> {
    let mut buffer: Vec<u16> = Vec::new();
    loop {
        let chara = file.read_u16::<LE>()?;
        if chara == 0 {
            break;
        };
        buffer.push(chara);
    }
    Ok(String::from_utf16(&buffer)?)
}

#[derive(BinRead)]
#[br(little)]
enum Sir0Type {
    #[br(magic = 4u32)]
    Type4,
    #[br(magic = 5u32)]
    Type5,
}

#[derive(BinRead)]
#[br(magic = b"FARC", little)]
struct FarcHeader {
    _unk_1: [u8; 0x1C],
    _sir0_type: Sir0Type,
    sir0_offset: u32,
    sir0_lenght: u32,
    all_data_offset: u32,
    _lenght_of_all_data: u32,
}

#[derive(Debug)]
/// A parser for a file in the farc format (see the crate root documentation for more information)
pub struct Farc<F: Read + Seek> {
    file: Arc<Mutex<F>>,
    index: FileNameIndex,
}

impl<F: Read + Seek> Farc<F> {
    /// Create and parse a new ``Farc`` object, with the specified input file
    pub fn new(mut file: F) -> Result<Self, FarcError> {
        let farc_header: FarcHeader = file.read_le().map_err(FarcError::ReadHeaderError)?;
        let file = Arc::new(Mutex::new(file));

        let sir0_partition = PartitionMutex::new(
            file.clone(),
            u64::from(farc_header.sir0_offset),
            u64::from(farc_header.sir0_lenght),
        )
        .map_err(FarcError::PartitionCreationError)?;
        let mut sir0 = Sir0::new(sir0_partition).map_err(FarcError::CreateSir0Error)?;
        let h = sir0.get_header();
        if h.len() < 12 {
            return Err(FarcError::Sir0HeaderNotLongEnought(h.len()));
        };
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
                u64::from(sir0_data_offset) + u64::from(file_index * entry_lenght),
            ))?;
            let filename_offset_or_hash = sir0_file.read_u32::<LE>()?;
            let data_offset = sir0_file.read_u32::<LE>()?;
            let data_length = sir0_file.read_u32::<LE>()?;

            let data_start = farc_header
                .all_data_offset
                .checked_add(data_offset)
                .map_or_else(
                    || {
                        Err(FarcError::DataStartOverflow(
                            farc_header.all_data_offset,
                            data_offset,
                        ))
                    },
                    Ok,
                )?;

            match sir0_fat5_type {
                0 => {
                    sir0_file.seek(SeekFrom::Start(u64::from(filename_offset_or_hash)))?;
                    let name = read_null_terminated_utf16_string(&mut sir0_file)?;
                    index.add_file_with_name(name, data_start, data_length)?;
                }
                1 => index.add_file_with_hash(filename_offset_or_hash, data_start, data_length)?,
                x => return Err(FarcError::UnsuportedFat5Type(x)),
            };
        }

        Ok(Self { file, index })
    }

    /// return the number of file contained in this ``Farc`` file
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.index.len()
    }

    /// return the number of file with an unknown name in this ``Farc`` file
    #[must_use]
    pub fn file_unknown_name(&self) -> usize {
        self.index.iter().filter(|f| f.name.is_none()).count()
    }

    /// return the number of file with a known name in this ``Farc`` file
    #[must_use]
    pub fn file_known_name(&self) -> usize {
        self.index.iter().filter(|f| f.name.is_some()).count()
    }

    /// iter over the known name of file
    pub fn iter_name(&self) -> impl Iterator<Item = &String> {
        self.index.iter().filter_map(|e| e.name.as_ref())
    }

    /// iter over all the hash without an occording known name
    pub fn iter_hash_unknown_name(&self) -> impl Iterator<Item = &u32> {
        self.index.iter().filter_map(|e| {
            if e.name.is_some() {
                None
            } else {
                Some(&e.name_hash)
            }
        })
    }

    /// iterate over all the known file, with their hash and (optionaly) their name.
    pub fn iter(&self) -> impl Iterator<Item = (u32, Option<&String>)> {
        self.index.iter().map(|f| (f.name_hash, f.name.as_ref()))
    }

    /// Iter over all the hash
    pub fn iter_all_hash(&self) -> impl Iterator<Item = &u32> {
        self.index.iter().map(|e| &e.name_hash)
    }

    /// Return an handle to a file stored in this ``Farc``, from it's name. It will hash the name as necessary.
    pub fn get_named_file(&self, name: &str) -> Result<PartitionMutex<F>, FarcError> {
        let file_data = match self.index.get_file_by_name(name) {
            Some(value) => value,
            None => return Err(FarcError::NamedFileNotFound(name.to_string())),
        };
        self.create_partition_from_data(file_data)
    }

    /// Return an handle to a file, whether its name is known or not.
    pub fn get_hashed_file(&self, hash: u32) -> Result<PartitionMutex<F>, FarcError> {
        let file_data = match self.index.get_file_by_hash(hash) {
            Some(value) => value,
            None => return Err(FarcError::HashedFileNotFound(hash)),
        };
        self.create_partition_from_data(file_data)
    }

    fn create_partition_from_data(
        &self,
        file_data: &FarcFile,
    ) -> Result<PartitionMutex<F>, FarcError> {
        PartitionMutex::new(
            self.file.clone(),
            u64::from(file_data.start),
            u64::from(file_data.length),
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
