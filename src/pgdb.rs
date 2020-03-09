use io_partition::clone_into_vec;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::string::FromUtf16Error;
use std::error::Error;
use std::fmt::Display;
use std::fmt;

#[derive(Debug)]
/// Any error that can be retourned by ``Pgdb``
pub enum PgdbError {
    /// An error happened while perfoming an IO
    IOError(io::Error),
    /// An error happened while parsing an utf-16 String
    FromUtf16Error(FromUtf16Error),
}

impl Error for PgdbError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IOError(err) => Some(err),
            Self::FromUtf16Error(err) => Some(err)
        }
    }
}

impl Display for PgdbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IOError(_) => write!(f, "An error happened while performing an IO"),
            Self::FromUtf16Error(_) => write!(f, "An error happened while parsing an utf-16 String"),
        }
    }
}

impl From<io::Error> for PgdbError {
    fn from(err: io::Error) -> Self {
        Self::IOError(err)
    }
}

impl From<FromUtf16Error> for PgdbError {
    fn from(err: FromUtf16Error) -> Self {
        Self::FromUtf16Error(err)
    }
}

fn pgdb_read_u32<T: Read + Seek>(file: &mut T) -> Result<u32, io::Error> {
    let mut buf = [0; 4];
    file.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn pgdb_read_u16<T: Read + Seek>(file: &mut T) -> Result<u16, io::Error> {
    let mut buf = [0; 2];
    file.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn pgdb_read_utf16_null_terminated_string<T: Read + Seek>(
    file: &mut T,
) -> Result<String, PgdbError> {
    let mut buffer: Vec<u16> = Vec::new();
    loop {
        let chara = pgdb_read_u16(file)?;
        if chara == 0 {
            break;
        };
        buffer.push(chara);
    }
    Ok(String::from_utf16(&buffer)?)
}

fn pgdb_read_utf16_at_offset<T: Read + Seek>(
    file: &mut T,
    offset: SeekFrom,
) -> Result<String, PgdbError> {
    file.seek(offset)?;
    pgdb_read_utf16_null_terminated_string(file)
}

#[derive(Debug)]
/// An entrie of a Pgdb file.
pub struct PGDBEntrie {
    pub primary_bgrs_filename: String,
    pub secondary_bgrs_filename: String,
    pub actor_name: String,
    pub data: Vec<u8>,
}

impl PGDBEntrie {
    /// Create a new ``PGDBEntrie`` with the input value
    pub fn new(
        actor_name: String,
        primary_bgrs_filename: String,
        secondary_bgrs_filename: String,
        data: Vec<u8>,
    ) -> Self {
        PGDBEntrie {
            actor_name,
            primary_bgrs_filename,
            secondary_bgrs_filename,
            data,
        }
    }
}

/// A parser for the Pgdb file format. See the crate root for more information
pub struct Pgdb {
    entries: Vec<PGDBEntrie>,
}

impl Pgdb {
    /// Create a new ``Pgdb`` and parse the input file
    pub fn new<T: Read + Seek>(mut file: T) -> Result<Self, PgdbError> {
        file.seek(SeekFrom::Start(4))?;
        // 0x04
        let sub_header_pointer = pgdb_read_u32(&mut file)?;
        // 0x08
        let sir0_relative_pointer_offset = pgdb_read_u32(&mut file)?;

        file.seek(SeekFrom::Start(sub_header_pointer as u64 + 4))?;
        // sub_header_pointer + 0x04
        let data_offset = pgdb_read_u32(&mut file)?;
        // sub_header_pointer + 0x08
        let num_entries = pgdb_read_u32(&mut file)?;

        let data_size = sir0_relative_pointer_offset - data_offset;
        let entry_size = ((data_size as f64) / (num_entries as f64)).floor() as u32;

        let mut entries = Vec::new();

        for count in 0..(num_entries - 1) {
            let entry_offset = (data_offset + count * entry_size) as u64;
            file.seek(SeekFrom::Start(entry_offset))?;
            let primary_bgrs_filename_pointer = pgdb_read_u32(&mut file)?;
            let secondary_bgrs_filename_pointer = pgdb_read_u32(&mut file)?;
            let actor_name_pointer = pgdb_read_u32(&mut file)?;

            let data_lenght = entry_size - 12;
            let data_offset = entry_offset + 12;

            let data = clone_into_vec(&mut file, data_offset, data_lenght as u64)?;
            let primary_bgrs_filename = pgdb_read_utf16_at_offset(
                &mut file,
                SeekFrom::Start(primary_bgrs_filename_pointer as u64),
            )?;

            let secondary_bgrs_filename = pgdb_read_utf16_at_offset(
                &mut file,
                SeekFrom::Start(secondary_bgrs_filename_pointer as u64),
            )?;
            let actor_name =
                pgdb_read_utf16_at_offset(&mut file, SeekFrom::Start(actor_name_pointer as u64))?;

            entries.push(PGDBEntrie::new(
                actor_name,
                primary_bgrs_filename,
                secondary_bgrs_filename,
                data,
            ));
        }

        Ok(Self { entries })
    }

    /// return the list of ``PGDBEntrie`` contained in this ``PGDB``
    pub fn get_entries(&self) -> &Vec<PGDBEntrie> {
        &self.entries
    }
}
