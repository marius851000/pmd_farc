use byteorder::{WriteBytesExt, LE};
use io::{copy, SeekFrom};
use pmd_sir0::{write_sir0_footer, write_sir0_header, Sir0WriteFooterError};
use thiserror::Error;

use crate::{Farc, FarcError};
use std::io::{Read, Seek, Write};
use std::{
    collections::HashMap,
    convert::TryInto,
    io::{self, Cursor},
    num::TryFromIntError,
};

#[derive(Error, Debug)]
/// An error that could happen with any function of a FarcWriter
pub enum FarcWriterError {
    /// An [`io::Error`] occured
    #[error("input/output error")]
    IOError(#[from] io::Error),
    /// An error occured while constructing/writing the sir0 footer
    #[error("sir0 write footer error")]
    Sir0WriteFooterError(#[from] Sir0WriteFooterError),
    /// A [`FarcError`] occured
    #[error("an error originated from the Farc this struct is build from")]
    FarcError(#[from] FarcError),
    /// Too much content are tried to be compressed resulting in an (probably) u32 overflow.
    #[error("The archive is too big. There may be a number of limiting factor. This is usually caused if the result file would take more than 4GiB. You should remove or reduce the size of big files...")]
    TooBig(#[from] TryFromIntError), // alia to TryFromIntError for convenience
}

#[derive(Default, Debug)]
/// Represent the content to be written to a FARC file. IT can only create hash-indexed file.
pub struct FarcWriter {
    hashed_files: HashMap<u32, Vec<u8>>,
}

impl FarcWriter {
    /// Create a new [`FarcWriter`] from an extracted [`Farc`] file
    pub fn new_from_farc<FT: Read + Seek>(farc: &Farc<FT>) -> Result<Self, FarcWriterError> {
        let mut farc_writer = Self::default();

        for file_hash in farc.iter_all_hash() {
            let mut file = farc.get_hashed_file(*file_hash)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            farc_writer.add_hashed_file(*file_hash, buffer);
        }

        Ok(farc_writer)
    }

    /// Add a file to be written with the given hash (as definied in the [`hash_name`] documentation)
    pub fn add_hashed_file(&mut self, hash: u32, content: Vec<u8>) {
        self.hashed_files.insert(hash, content);
    }

    /// Write an hashed Farc file to the given writer, with the content of this struct
    pub fn write_hashed<T: Write + Seek>(&self, file: &mut T) -> Result<(), FarcWriterError> {
        // sort the hash, as this is a binary tree search
        let mut hash_sorted = self.hashed_files.iter().collect::<Vec<_>>();
        hash_sorted.sort();

        let mut storage_file: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        let mut meta_file: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        meta_file.write_all(&[0; 12])?; // reserve sir0 header space
        meta_file.write_all(&[0; 4])?; // 0x10 padding
        let mut meta_pointer = vec![4, 8];

        for (file_hash, file_content) in hash_sorted {
            let file_start = storage_file.position();
            let file_lenght = file_content.len();
            storage_file.write_all(file_content)?;
            if storage_file.position() % 16 != 0 {
                storage_file.write_all(&vec![0; storage_file.position() as usize % 16])?;
            };
            meta_file.write_u32::<LE>(*file_hash)?;
            //TODO: check transformation, resulting in error for too big file
            meta_file.write_u32::<LE>(file_start.try_into()?)?;
            //TODO: idem as upper
            meta_file.write_u32::<LE>(file_lenght.try_into()?)?;
        }

        meta_pointer.push(meta_file.position().try_into()?);

        if meta_file.position() % 16 != 0 {
            meta_file.write_all(&vec![0; 16 - meta_file.position() as usize % 16])?;
        };

        let sir0_header_position = meta_file.position().try_into()?;
        meta_file.write_u32::<LE>(0x10)?; // the start of the sir0 data
        meta_file.write_u32::<LE>(self.hashed_files.len().try_into()?)?; // number of file //TODO: overflow (unlikely to happen actually)
        meta_file.write_u32::<LE>(1)?; // meta type -- 1 for hashed name

        if meta_file.position() % 16 != 0 {
            meta_file.write_all(&vec![0; 16 - meta_file.position() as usize % 16])?;
        };

        let sir0_footer_position = meta_file.position().try_into()?;
        write_sir0_footer(&mut meta_file, &meta_pointer)?;

        if meta_file.position() % 16 != 0 {
            meta_file.write_all(&vec![0; 16 - meta_file.position() as usize % 16])?;
        };

        meta_file.seek(SeekFrom::Start(0))?;
        write_sir0_header(&mut meta_file, sir0_header_position, sir0_footer_position)?;

        //TODO: check for padding after the sir0 file

        let meta_file_lenght = meta_file.seek(SeekFrom::End(0))?.try_into()?;
        let storage_file_lenght: u32 = storage_file.seek(SeekFrom::End(0))?.try_into()?;
        let no_padding_storage_start = 0x80 + meta_file_lenght;
        let padding_size_storage_start = if no_padding_storage_start % 256 != 0 {
            256 - no_padding_storage_start % 256
        } else {
            0
        };

        let storage_start = no_padding_storage_start + padding_size_storage_start;

        file.write_all(b"FARC")?; //0x0, magic
        file.write_u32::<LE>(0)?; //0x4, unknown
        file.write_u32::<LE>(0)?; //0x8, idem
        file.write_u32::<LE>(2)?; //0xC, idem
        file.write_u32::<LE>(0)?; //0x10, idem
        file.write_u32::<LE>(0)?; //0x14, idem
        file.write_u32::<LE>(7)?; //0x18, idem
        file.write_all(&[0xA4, 0x3C, 0xEA, 0x77])?; //0x1C, idem
        file.write_u32::<LE>(5)?; //0x20, sir 0 type
        file.write_u32::<LE>(0x80)?; //0x24, offset of the start of the sir0 file
        file.write_u32::<LE>(meta_file_lenght)?; //0x28, the lenght of the sir0 file.
        file.write_u32::<LE>(storage_start)?; //0x2C, the offset of the true data.
        file.write_u32::<LE>(storage_file_lenght + 112)?; //0x30, the lenght of the true data
                                                          //TODO: why +112
        file.write_all(&[0; 0x80 - 0x34])?; //0x34 -- padding

        meta_file.seek(SeekFrom::Start(0))?;
        copy(&mut meta_file, file)?;

        file.write_all(&vec![0; padding_size_storage_start as usize])?;

        storage_file.seek(SeekFrom::Start(0))?;
        copy(&mut storage_file, file)?;

        Ok(())
    }
}
