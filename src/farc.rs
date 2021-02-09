use crate::{FarcFile, FileNameIndex};
use binread::{derive_binread, BinReaderExt};
use byteorder::{ReadBytesExt, LE};
use io_partition::PartitionMutex;
use pmd_sir0::{Sir0, Sir0Error};
use std::io;
use std::io::{Read, Seek, SeekFrom};
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

#[derive_binread]
#[br(little)]
enum Sir0Type {
    #[br(magic = 4u32)]
    Type4,
    #[br(magic = 5u32)]
    Type5,
}

#[derive_binread]
#[br(magic = b"FARC", little)]
struct FarcHeader {
    _unk_1: [u8; 0x1C],
    _sir0_type: Sir0Type,
    sir0_offset: u32,
    sir0_lenght: u32,
    all_data_offset: u32,
    _unk_2: u32,
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
            farc_header.sir0_offset as u64,
            farc_header.sir0_lenght as u64,
        )
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
            let filename_offset_or_hash = sir0_file.read_u32::<LE>()?;
            let data_offset = sir0_file.read_u32::<LE>()?;
            let data_length = sir0_file.read_u32::<LE>()?;

            match sir0_fat5_type {
                0 => {
                    sir0_file.seek(SeekFrom::Start(filename_offset_or_hash as u64))?;
                    let name = read_null_terminated_utf16_string(&mut sir0_file)?;
                    index.add_file_with_name(
                        name,
                        farc_header.all_data_offset + data_offset,
                        data_length,
                    );
                }
                1 => index.add_file_with_hash(
                    filename_offset_or_hash,
                    farc_header.all_data_offset + data_offset,
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
