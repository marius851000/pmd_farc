use pmd_farc::{Farc, FarcWriter};
use std::fs::File;
use std::path::PathBuf;

pub fn main() {
    let file = File::open(PathBuf::from("./message.bin")).unwrap();
    let farc = match Farc::new(file) {
        Ok(v) => v,
        Err(e) => {
            match e {
                pmd_farc::FarcError::ReadHeaderError(e) => match e {
                    binread::Error::NoVariantMatch { pos } => {
                        println!("{}", pos);
                    }
                    _ => panic!("{:?}", e),
                },
                _ => panic!(),
            }
            panic!();
        }
    };
    let farc_writer = FarcWriter::new_from_farc(&farc).unwrap();
    let mut out = File::create(PathBuf::from("./out.bin")).unwrap();
    farc_writer.write_hashed(&mut out).unwrap();
}
