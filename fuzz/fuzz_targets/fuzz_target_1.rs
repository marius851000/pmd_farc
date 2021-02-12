#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;
use pmd_farc::{Farc, FarcWriter};
use std::io::{Seek, SeekFrom};

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);
    if let Ok(farc) = Farc::new(&mut cursor) {
        for name in farc.iter_name() {
            let _ = farc.get_named_file(name);
        }
        let mut failed = false;
        for hash in farc.iter_all_hash() {
            if farc.get_hashed_file(*hash).is_err() {
                failed = true;
            };
        };
        if !failed {
            let mut write_file = Cursor::new(Vec::new());
            let farc_writer = FarcWriter::new_from_farc(&farc).unwrap();
            farc_writer.write_hashed(&mut write_file).unwrap();
            write_file.seek(SeekFrom::Start(0)).unwrap();
            let newly_parsed = Farc::new(&mut write_file).unwrap();
            assert_eq!(newly_parsed.file_count(), farc.file_count());
        }
    }
});
