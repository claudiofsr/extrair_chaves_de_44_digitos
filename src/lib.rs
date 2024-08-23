mod args;

pub use self::args::*;

use claudiofsr_lib::{open_file, BytesExtension, StrExtension};
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use regex::Regex;
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read},
    ops::Deref,
    path::{Path, PathBuf},
    str,
    sync::LazyLock,
};
use walkdir::{DirEntry, WalkDir};

pub const NEWLINE_BYTE: u8 = b'\n';
pub const DELIMITER_CHAR: char = '|';

pub static REGEX_CHAVE44: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?ix)
        (\b|\D)\d{44}(\b|\D)
    ",
    )
    .unwrap()
});

pub fn get_efd_entries(arguments: &Arguments) -> Result<Vec<DirEntry>, Box<dyn std::error::Error>> {
    let dir_path = get_path(&arguments.path)?;

    let entries: Vec<DirEntry> = WalkDir::new(dir_path)
        .min_depth(arguments.min_depth)
        .max_depth(arguments.max_depth)
        .into_iter()
        .flatten() // Result<DirEntry, Error> to DirEntry
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
        })
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|s| s.to_uppercase().starts_with("PISCOFINS"))
        })
        .collect();

    Ok(entries)
}

/// Get path from arguments or from default (current directory).
pub fn get_path(opt_path: &Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let relative_path: PathBuf = match opt_path {
        Some(path) => path.to_owned(),
        None => PathBuf::from("."),
    };

    Ok(relative_path)
}

pub fn get_map(entry: &DirEntry) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let path = entry.path();
    let file = open_file(path)?;
    let buffer = BufReader::new(file);

    let mut chaves: BTreeSet<String> = BTreeSet::new();

    buffer
        .split(NEWLINE_BYTE)
        .flatten()
        .enumerate()
        .map(|(line_number, vec_bytes)| get_string_utf8(vec_bytes.trim(), line_number + 1, path))
        .map(split_line)
        .filter(|campos| campos.len() >= 2)
        .take_while(|campos| campos[0] != "9999")
        .for_each(|campos| {
            for campo in campos {
                for cap in REGEX_CHAVE44.captures_iter(&campo) {
                    let chave = cap[0].to_string().remove_non_digits();
                    chaves.insert(chave);
                }
            }
        });

    Ok(chaves)
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
        Err(error1) => {
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
                eprintln!("Failed to convert data from WINDOWS_1252 to UTF-8!");
                panic!("Error: {error1}\n{error2}\n");
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
    use extrair_chaves_de_44_digitos::split_line;
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
