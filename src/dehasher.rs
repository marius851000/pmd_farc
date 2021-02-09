/// This enum store the way we can find the name of the files of the compressed file
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FileHashType {
    /// In can this file contain files that have translated text. The game include debug information, in the form of an adjacent .lst file, that allow to know their name.
    /// See [`message_dehasher`] for function allowing to parse them
    Message,
}

impl FileHashType {
    /// Try to find the way to get the name of files in the archive, based on the archive name
    ///
    /// Return None if the method can't be found or is not implemented
    ///
    /// # Example
    /// ```
    /// use pmd_farc::FileHashType;
    /// assert_eq!(FileHashType::predict_from_file_name("message.bin"), Some(FileHashType::Message));
    /// assert_eq!(FileHashType::predict_from_file_name("unknown.bin"), None);
    /// ```
    pub fn predict_from_file_name(file_name: &str) -> Option<FileHashType> {
        match file_name {
            "message.bin"
            | "message_en.bin"
            | "message_fr.bin"
            | "message_ge.bin"
            | "message_it.bin"
            | "message_sp.bin"
            | "message_us.bin"
            | "message_debug.bin"
            | "message_debug_en.bin"
            | "message_debug_fr.bin"
            | "message_debug_ge.bin"
            | "message_debug_it.bin"
            | "message_debug_sp.bin"
            | "message_debug_us.bin" => Some(FileHashType::Message),
            _ => None,
        }
    }
}

/// contain useful function to get the original name of message* farc files.
pub mod message_dehash {
    use crate::Farc;
    use std::io;
    use std::io::{Read, Seek};

    /// get the file name of the list file that should be openened
    ///
    /// # Example
    /// ```
    /// use pmd_farc::message_dehash::get_file_name;
    /// assert_eq!(&get_file_name("message.bin").unwrap(), Some("message.lst").unwrap());
    /// ```
    pub fn get_file_name(original_file_name: &str) -> Option<String> {
        Some(original_file_name.split(".").next()?.to_string() + ".lst")
    }

    /// Try to find the name of files in a farc based on a an input file file, that contain lines of path
    /// (sometimes found as adjacent .lst files to .bin files)
    ///
    /// the expected lst file may be found with [`message_dehash::get_file_name`].
    pub fn try_possible_name<F: Read, FT: Read + Seek>(
        farc: &mut Farc<FT>,
        list_file: &mut F,
    ) -> Result<(), io::Error> {
        let mut strings = String::new();
        list_file.read_to_string(&mut strings)?;

        for line in strings.split("\n") {
            if line == "" {
                continue;
            };
            match line.split("/").last() {
                Some(line) => {
                    if !farc.check_file_name(line) {
                        debug!(
                            "the file name {} can't be found in a message farc archive",
                            line
                        );
                    };
                }
                None => (),
            };
        }
        Ok(())
    }
}
