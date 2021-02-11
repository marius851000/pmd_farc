#![warn(missing_docs)]
//! This library permit to have a read-only access to Farc file format used in the 3ds game of pokemon mystery dungeon.
//!
//! The ``pmd_farc::Farc`` file format is a packed file format, like tar. It doesn't have a notion of subdirectory. There is two type of ``pmd_farc::Farc`` file:
//! - A version with file index by their name.
//! - A version with file index by the crc32 hash of their name.
//! This library automatically identify the ``pmd_farc::Farc`` type. For type without full file name, you can test if a ``String`` correspond to a file name.

#[macro_use]
extern crate log;

mod farc;
pub use farc::{Farc, FarcError};

mod dehasher;
pub use dehasher::message_dehash;
pub use dehasher::FileHashType;


mod file_name_index;
pub use file_name_index::{hash_name, FileNameError, FileNameIndex};

mod farc_file;
pub use farc_file::FarcFile;