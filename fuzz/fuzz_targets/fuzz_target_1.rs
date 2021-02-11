#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;
use pmd_farc::Farc;
use std::io::{Read, Seek, SeekFrom};
use io_partition::PartitionMutex;

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);
    if let Ok(farc) = Farc::new(&mut cursor) {
        fn test_file<T: Read + Seek>(mut file: PartitionMutex<T>) {
            let mut file = file.lock().unwrap();
            let lenght = file.seek(SeekFrom::End(0)).unwrap();
            let mut buffer = vec![0; lenght as usize];
            file.seek(SeekFrom::Start(0)).unwrap();
            file.read(&mut buffer).unwrap();
            file.seek(SeekFrom::Start(0)).unwrap();
            file.read_exact(&mut buffer).unwrap();
        }
        for name in farc.iter_name() {
            let _ = farc.get_named_file(name).map(|f| test_file(f));
        }
        for hash in farc.iter_all_hash() {
            let _ = farc.get_hashed_file(*hash).map(|f| test_file(f));
        }
        //TODO: once the writer is finished, rewrite, then reparse it with no error at all allowed.
    }
});
