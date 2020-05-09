#![warn(missing_docs)]
//! This library permit to have a read-only access to Farc and Pgdb file format used in the 3ds game of pokemon mystery dungeon.
//!
//! The ``pmd_farc::Farc`` file format is a packed file format, like tar. It doesn't have a notion of subdirectory. There is two type of ``pmd_farc::Farc`` file:
//! - A version with file index by their name.
//! - A version with file index by the crc32 hash of their name.
//! This library automatically identify the ``pmd_farc::Farc`` type. For type without full file name, you can test if a ``String`` correspond to a file name.
//! There is some unfinished helper function you can use to lock for the name of the files by various way. As it is finished, it is not included in the library, but it can be found at pmd_farc/src/find_name.rs.
//!
//! The ``pmd_farc::Pgdb`` contain some data about the file "pokemon_graphic.bin", including file names (usefull, as the file name in "pokemon_graphic.bin" are hashed)

#[macro_use]
extern crate log;

mod farc;
pub use farc::{Farc, FarcError};

mod pgdb;
pub use pgdb::{Pgdb, PgdbError};

mod dehasher;
pub use dehasher::FileHashType;
pub use dehasher::message_dehash;

// unused function to find hashed file name
/*
mod find_name;
pub use find_name::{find_name_monster_graphic, GetNameMonsterGraphicError};
*/
