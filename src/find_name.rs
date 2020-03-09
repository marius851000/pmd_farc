//! unfinished, unused code

/* use crate::{Farc, FarcError, Pgdb};
use pmd_bch::{Bch, BchError};
use pmd_bgrs::{Bgrs, BgrsError};
use std::io;
use std::io::{Read, Seek};

//TODO: Error & Display
#[derive(Debug)]
pub enum GetNameMonsterGraphicError {
    GetSubFileError(FarcError),
    IOError(io::Error),
    CreateBchError(BchError),
}

impl From<io::Error> for GetNameMonsterGraphicError {
    fn from(err: io::Error) -> GetNameMonsterGraphicError {
        GetNameMonsterGraphicError::IOError(err)
    }
}

fn read_null_terminated_ascii_string<T: Read>(
    file: &mut T,
) -> Result<String, GetNameMonsterGraphicError> {
    let mut buf = [0];
    let mut result = String::new();
    loop {
        file.read_exact(&mut buf)?;
        let chara = buf[0];
        if chara == 0 {
            return Ok(result);
        } else {
            result.push(chara as char)
        }
    }
}

pub fn find_name_monster_graphic<T: Read + Seek>(
    farc: &mut Farc<T>,
    pgdb: &mut Pgdb,
) -> Result<(), GetNameMonsterGraphicError> {
    let entries = pgdb.get_entries();

    let primary = entries
        .iter()
        .map(|entrie| entrie.primary_bgrs_filename.clone());
    let secondary = entries.iter().filter_map(|entrie| {
        if entrie.secondary_bgrs_filename == "" {
            None
        } else {
            Some(entrie.secondary_bgrs_filename.clone() + ".bgrs")
        }
    });
    let file_names = primary.chain(secondary);

    farc.check_file_name_iter(file_names); //TODO: error

    // check for missed BGRS file
    for file_hash in farc.iter_hash() {
        let mut actual_file = farc
            .get_unnamed_file(&file_hash)
            .map_err(|err| GetNameMonsterGraphicError::GetSubFileError(err))?;
        let bgrs = match Bgrs::new(actual_file) {
            Ok(value) => value,
            Err(BgrsError::InvalidMagic(_)) => continue,
            Err(err) => panic!(err), //TODO:
        };
        let full_name = bgrs.get_name().clone() + ".bgrs".into();
        farc.check_file_name(&full_name);
    }

    // Infer BCH files from BGRS
    let mut bch_file_name = Vec::new();
    for file_name in farc.iter_name().filter(|name| name.ends_with(".bgrs")) {
        let actual_file = farc
            .get_named_file(file_name)
            .map_err(|err| GetNameMonsterGraphicError::GetSubFileError(err))?;
        let bgrs = Bgrs::new(actual_file).unwrap(); //TODO:
        for animation in bgrs.iter_animations() {
            if animation.get_name() == "" {
                continue;
            }
            bch_file_name.push(animation.get_name().clone() + ".bchmata");
            bch_file_name.push(animation.get_name().clone() + ".bchskla");
        }
    }
    farc.check_file_name_iter(bch_file_name.iter().cloned());

    // Find the missing BCH name
    let mut missed_bch_file_name = Vec::new();
    for file_hash in farc.iter_hash() {
        let actual_file = farc
            .get_unnamed_file(&file_hash)
            .map_err(|err| GetNameMonsterGraphicError::GetSubFileError(err))?;

        let bch = match Bch::new(actual_file) {
            Ok(value) => value,
            Err(BchError::InvalidHeader(header)) => {
                println!("{}", header);
                panic!()
            }
            Err(err) => return Err(GetNameMonsterGraphicError::CreateBchError(err)),
        };
        for string in bch.get_strings() {
            missed_bch_file_name.push(string.clone() + ".bchmata");
            missed_bch_file_name.push(string.clone() + ".bchskla");
        }
    }
    farc.check_file_name_iter(missed_bch_file_name.iter().cloned());

    //TODO: correct it with the help of evandixon

    Ok(())
}
*/
