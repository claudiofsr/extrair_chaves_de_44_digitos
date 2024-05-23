mod args;

pub use self::args::*;

use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    io::Read, 
    ops::Deref, 
    path::{Path, PathBuf}, 
    str,
};
use walkdir::{DirEntry, WalkDir};
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;

pub const NEWLINE_BYTE: u8 = b'\n';
pub const DELIMITER_CHAR: char = '|';

pub static REGEX_CHAVE44: Lazy<Regex> = Lazy::new(||
    Regex::new(r"(?ix)
        (\b|\D)\d{44}(\b|\D)
    ").unwrap()
);

/// Get path from arguments or from default (current directory).
pub fn get_path(opt_path: &Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let relative_path: PathBuf = match opt_path {
        Some(path) => {
            if std::path::Path::new(&path).try_exists()? {
                path.to_path_buf()
            } else {
                eprintln!("The path {path:?} was not found!");
                panic!("fn get_path()");
            }
        }
        None => PathBuf::from("."),
    };

    Ok(relative_path)
}

pub fn get_efd_entries(arguments: &Arguments) -> Result<Vec<DirEntry>, Box<dyn std::error::Error>> {
    let dir_path = get_path(&arguments.path)?;

    let max_depth: usize = match arguments.max_depth {
        Some(depth) => depth,
        None => std::usize::MAX,
    };

    let entries: Vec<DirEntry> = WalkDir::new(dir_path)
        .max_depth(max_depth)
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry|
            entry.path().extension().is_some_and(|ext|
                ext.eq_ignore_ascii_case("txt")
            )
        )
        .filter(|entry|
            entry.path().file_name().is_some_and(|os_str| 
                os_str.to_str().is_some_and(|f| f.to_uppercase().starts_with("PISCOFINS"))
            )
        )
        .collect();

    Ok(entries)
}

/// Converts a slice of bytes to a String.
///
/// Consider the case of files with differently encoded lines!
///
/// That is, one line in UTF-8 and another line in WINDOWS_1252.
pub fn get_string_utf8<P>(slice_bytes: &[u8], line_number: usize, filename: P) -> String
where
    P: AsRef<Path> + std::fmt::Debug,
{
    // from_utf8() checks to ensure that the bytes are valid UTF-8
    match str::from_utf8(slice_bytes) {
        Ok(str) => str.to_string(),
        Err(_) => {
            let mut data = DecodeReaderBytesBuilder::new()
                .encoding(Some(WINDOWS_1252))
                .build(slice_bytes);
            let mut buffer = String::new();
            if let Err(error2) = data.read_to_string(&mut buffer) {
                eprintln!("Problem reading data from file in buffer!");
                eprintln!("File: {filename:?}");
                eprintln!("Line nº {line_number}");
                eprintln!("Used encoding type: WINDOWS_1252.");
                eprintln!("Try another encoding type!");
                panic!(
                    "Failed to convert data from WINDOWS_1252 to UTF-8!\nError: {error2}\n",
                );
            }
            buffer
        }
    }
}

// cargo doc --open
// https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html
/**
Split the Line using the dilimiter
```
    use efd_contribuicoes::split_line;
    let line = " | campo1| campo2 | ...... |campoN | ";
    let campos = split_line(line);
    // As a result, we will have:
    let result: Vec<String> = vec![
        "campo1".to_string(),
        "campo2".to_string(),
        "......".to_string(),
        "campoN".to_string(),
    ];
    assert_eq!(result, campos);
```
*/
pub fn split_line<T>(line: T) -> Vec<String>
where
    T: Deref<Target = str>,
{
    let mut campos: Vec<String> = line
        .split(DELIMITER_CHAR)
        .skip(1)
        .map(|campo| campo.trim().to_string())
        .collect();

    campos.pop();

    campos
}