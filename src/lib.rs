mod args;
mod error;

pub use self::{
    args::*,
    error::{MyError, MyResult},
};

use claudiofsr_lib::open_file;
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use rayon::prelude::*;
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

/// Newline byte constant for file processing.
pub const NEWLINE_BYTE: u8 = b'\n';

/// Delimiter character for splitting lines.
pub const DELIMITER_CHAR: char = '|';

/// Lazy-initialized regex to find 44-digit keys.
/// It looks for 44 digits surrounded by word boundaries or non-digit characters.
/// The surrounding parts are non-capturing groups.
pub static REGEX_CHAVE44: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?ix)
        (?:\b|\D) # Non-capturing group for preceding boundary/non-digit
        (\d{44})  # Capturing group for the 44 digits
        (?:\b|\D) # Non-capturing group for trailing boundary/non-digit
    ",
    )
    .unwrap() // Regex compilation should not fail with a static string
});

/// Checa se uma DirEntry é um arquivo EFD Contribuições (arquivo .txt que começa com "PISCOFINS").
fn is_efd_contribuicoes_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file() // Deve ser um arquivo
        && entry
            .path()
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("txt")) // Extensão ".txt" (case-insensitive)
        && entry
            .file_name()
            .to_str()
            .is_some_and(|s| s.to_uppercase().starts_with("PISCOFINS")) // Nome começa com "PISCOFINS" (case-insensitive)
}

/// Retrieves a list of EFD (Escrituração Fiscal Digital) file entries.
///
/// Filters for files with a ".txt" extension and names starting with "PISCOFINS" (case-insensitive).
pub fn get_efd_entries(arguments: &Arguments) -> MyResult<Vec<DirEntry>> {
    let dir_path = get_path(&arguments.path)?;

    let entries: Vec<DirEntry> = WalkDir::new(dir_path)
        .min_depth(arguments.min_depth)
        .max_depth(arguments.max_depth)
        .into_iter()
        .map(|entry_result| {
            // Mapeia cada Result<DirEntry, walkdir::Error> para Result<Option<DirEntry>, walkdir::Error>
            // onde Some(entry) é para entradas que queremos manter
            // e None para entradas que não atendem aos filtros (mas não são erros)

            // Este `map` atua em cada Result<DirEntry, walkdir::Error>
            // Se for Err, ele propaga imediatamente via '?' ao final do 'collect'
            // Se for Ok(entry), ele continua a processar o 'entry'
            entry_result.map(|entry| {
                if is_efd_contribuicoes_file(&entry) {
                    Some(entry) // Entrada válida e filtrada
                } else {
                    None // Entrada válida, mas não passa nos filtros
                }
            })
        })
        // Transforma Result<Option<T>, E> em Option<Result<T>, E>.
        // Descarta 'None's (entradas não filtradas) e propaga 'Err's.
        .filter_map(Result::transpose)
        // Coleta os resultados em um Vec ou propaga
        // o primeiro walkdir::Error encontrado (convertido para MyError).
        .collect::<Result<Vec<DirEntry>, walkdir::Error>>()?;

    Ok(entries)
}

/// Get the directory path from arguments or default to the current directory.
pub fn get_path(opt_path: &Option<PathBuf>) -> MyResult<PathBuf> {
    let relative_path: PathBuf = match opt_path {
        Some(path) => path.to_owned(),
        None => PathBuf::from("."),
    };

    Ok(relative_path)
}

/// Processes all EFD (Escrituração Fiscal Digital) file entries in parallel
/// to extract and combine unique 44-digit keys into a single BTreeSet.
///
/// This function leverages Rayon for parallel processing and uses a functional
/// chain of iterators for robust error handling and efficient data aggregation.
///
/// # Arguments
/// * `efd_entries` - A slice of `DirEntry` references, each representing an EFD file.
///
/// # Returns
/// A `MyResult` containing a `BTreeSet<String>` of all unique 44-digit keys
/// found across all processed files. Returns `Err(MyError)` if any file
/// processing encounters an error.
pub fn process_all_efd_files_parallel(efd_entries: &[DirEntry]) -> MyResult<BTreeSet<String>> {
    // 1. Parallelize file processing:
    //    Converts the slice of DirEntry into a parallel iterator.
    let all_file_keys: BTreeSet<String> = efd_entries
        .into_par_iter()
        // 2. Map each DirEntry to its extracted keys:
        //    Calls `get_map` for each DirEntry, returning a `MyResult<BTreeSet<String>>`.
        //    `get_map` itself handles file I/O, decoding, and key extraction for a single file.
        .map(get_map)
        // 3. Collect results, handling errors:
        //    This `collect` method on an iterator of `Result<T, E>` will:
        //    - If all items are `Ok`, collect all `BTreeSet<String>` into a `Vec<BTreeSet<String>>`.
        //    - If any item is `Err`, it immediately returns the first encountered `MyError`,
        //      potentially cancelling further parallel computations.
        //    The `?` operator then propagates this error or unwraps the `Vec<BTreeSet<String>>`.
        .collect::<Result<Vec<BTreeSet<String>>, MyError>>()?
        // At this point, if no errors occurred, we have `Vec<BTreeSet<String>>`.
        // The subsequent steps aim to flatten this into a single `BTreeSet<String>`.
        // 4. Convert the `Vec` into a sequential iterator:
        //    Necessary to use `.flatten()` which operates on `IntoIterator`.
        .into_iter()
        // 5. Flatten the `Vec<BTreeSet<String>>` into an iterator of `String`:
        //    Combines all individual `BTreeSet`s into one continuous stream of key strings.
        .flatten()
        // 6. Collect all key strings into a single `BTreeSet`:
        //    Ensures all keys are unique and maintains them in sorted order.
        .collect();

    Ok(all_file_keys)
}

/// Processa uma única linha do arquivo, extraindo chaves ou sinalizando interrupção/ignorar.
///
/// Retorna:
/// - `Ok(Some(keys))`: Se chaves foram encontradas na linha.
/// - `Ok(None)`: Se a linha deve ser ignorada (ex: poucos campos).
/// - `Err(MyError::EofMarkerReached)`: Se "9999" foi encontrado (interrupção controlada).
/// - `Err(MyError::...)`: Para outros erros reais (decodificação, etc.).
fn process_line_for_keys(
    line_bytes: Vec<u8>,
    line_number: usize,
    file_path: &PathBuf,
) -> MyResult<Option<Vec<String>>> {
    let trimmed_bytes = line_bytes.trim_ascii();

    // Decode bytes to String, propagating `EncodingError`
    let line_string = get_string_utf8(trimmed_bytes, line_number, file_path)?;

    // Split the line into fields using a predefined delimiter
    let fields = split_line(line_string);

    // Se o primeiro campo é "9999", sinaliza para parar o processamento do arquivo.
    // 1. Take_while: Stop processing if the first field is "9999"
    if fields.first().is_some_and(|f| f == "9999") {
        // Signal a controlled, non-error termination
        return Err(MyError::EofMarkerReached(
            file_path.to_owned(), // Capture the file path
            line_number,          // Capture the line number where "9999" was found
        ));
    }

    // Filtra linhas com menos de 2 campos
    // 2. Filter: Skip lines with insufficient fields (less than 2)
    if fields.len() < 2 {
        return Ok(None); // Linha ignorada
    }

    let mut keys_on_line = Vec::new();

    // If filters are passed, process fields to extract keys
    for field_content in fields {
        for capture in REGEX_CHAVE44.captures_iter(&field_content) {
            // The first capturing group (index 1) contains the actual 44-digit key.
            if let Some(matched_key) = capture.get(1) {
                keys_on_line.push(matched_key.as_str().to_string());
            }
        }
    }
    Ok(Some(keys_on_line)) // Retorna as chaves encontradas nesta linha
}

/// Processes a directory entry (file) to extract unique 44-digit keys.
///
/// This function reads the content of a file specified by `entry` line by line,
/// decodes each line, splits it into fields. It collects unique 44-digit keys
/// found within these fields.
///
/// The processing stops upon encountering a line where the first field is "9999`,
/// treating this as a successful end-of-file marker. Real I/O or decoding errors
/// will propagate as `MyError`.
///
/// # Arguments
/// * `entry` - A reference to a `DirEntry` representing the file to process.
///
/// # Returns
/// A `MyResult` containing a `BTreeSet<String>` of unique 44-digit keys
/// found in the file. Returns `Err(MyError)` if file operations, decoding,
/// or other unexpected issues occur.
pub fn get_map_funcional(entry: &DirEntry) -> MyResult<BTreeSet<String>> {
    let path = entry.path();
    let file = open_file(path)?; // Propaga qualquer erro ao abrir o arquivo
    let buffer = BufReader::new(file);

    let mut collected_keys: BTreeSet<String> = BTreeSet::new();

    // `try_fold` é usado para iterar, acumular chaves e parar a iteração
    // se um erro (incluindo EofMarkerReached) for retornado pelo closure.
    // O estado inicial é `()` (não estamos acumulando nada no `try_fold` em si,
    // apenas no `collected_keys` mutável).
    let final_processing_status: Result<(), MyError> = buffer
        .split(NEWLINE_BYTE) // Iterator of `Result<Vec<u8>, io::Error>`
        .map(|byte_result| byte_result.map_err(MyError::IoError)) // Converte io::Error para MyError
        .enumerate() // Adiciona o índice da linha (0-based)
        .try_fold((), |_, (line_idx, line_bytes_result)| {
            let line_number = line_idx + 1; // Número da linha (1-based)

            // Tenta processar a linha. O resultado é um MyResult<Option<Vec<String>>>
            let keys_result: MyResult<Option<Vec<String>>> = match line_bytes_result {
                Ok(line_bytes) => {
                    process_line_for_keys(line_bytes, line_number, &path.to_path_buf())
                }
                Err(e) => Err(e), // Erro de I/O da linha é propagado diretamente
            };

            // Gerencia o resultado do processamento da linha
            match keys_result {
                Ok(Some(keys)) => {
                    for key in keys {
                        collected_keys.insert(key);
                    }
                    Ok(()) // Continua a iteração (Ok para try_fold)
                }
                Ok(None) => Ok(()), // Linha ignorada, continua (Ok para try_fold)
                Err(e) => Err(e), // Erro ou EofMarkerReached, interrompe a iteração (Err para try_fold)
            }
        });

    // FINAL RESULT HANDLING:
    // Differentiate between normal completion/controlled break and an actual error.
    match final_processing_status {
        Ok(_) => Ok(collected_keys), // Iteração completou sem nenhum Err retornado pelo try_fold
        Err(MyError::EofMarkerReached(..)) => Ok(collected_keys), // Interrompido por 9999 (sucesso)
        Err(e) => Err(e),            // Interrompido por um erro real
    }
}

pub fn get_map(entry: &DirEntry) -> MyResult<BTreeSet<String>> {
    let path = entry.path();
    let file = open_file(path)?; // Propaga qualquer erro ao abrir o arquivo
    let buffer = BufReader::new(file);

    let mut collected_keys: BTreeSet<String> = BTreeSet::new();

    // Iterar sobre as linhas, tratando erros e o marcador de fim.
    for (line_idx, byte_result) in buffer.split(NEWLINE_BYTE).enumerate() {
        let line_number = line_idx + 1; // Número da linha (1-based)

        let line_bytes: Vec<u8> = byte_result?;

        // Tenta processar a linha.
        // O `process_line_for_keys` já lida com "9999" e linhas para ignorar.
        match process_line_for_keys(line_bytes, line_number, &path.to_path_buf()) {
            Ok(Some(keys)) => {
                // Se encontrou chaves, insere-as no conjunto.
                for key in keys {
                    collected_keys.insert(key);
                }
            }
            Ok(None) => {
                // Linha ignorada, continua para a próxima.
                continue;
            }
            Err(MyError::EofMarkerReached(..)) => {
                // Marcador "9999" encontrado.
                // Como isso é considerado um "sucesso" para o processamento do arquivo,
                // simplesmente retornamos as chaves coletadas até agora.
                return Ok(collected_keys);
            }
            Err(e) => {
                // Outro erro real, propaga-o.
                return Err(e);
            }
        }
    }

    // Se o loop terminar sem encontrar "9999" ou outro erro,
    // significa que o arquivo foi processado até o fim.
    Ok(collected_keys)
}

/// Converts a slice of bytes to a String, attempting UTF-8 first, then WINDOWS_1252.
///
/// This handles files with mixed encodings by trying common encodings.
/// If both fail, it returns a `MyResult::Err(MyError::EncodingError)`.
pub fn get_string_utf8<P>(slice_bytes: &[u8], line_number: usize, filename: P) -> MyResult<String>
where
    P: AsRef<Path> + std::fmt::Debug,
{
    // Try to convert using UTF-8 first
    match str::from_utf8(slice_bytes) {
        Ok(s) => Ok(s.to_string()),
        Err(error1) => {
            // If UTF-8 fails, try WINDOWS_1252
            let mut data = DecodeReaderBytesBuilder::new()
                .encoding(Some(WINDOWS_1252))
                .build(slice_bytes);
            let mut buffer = String::new();

            match data.read_to_string(&mut buffer) {
                Ok(_) => Ok(buffer), // Successfully decoded with WINDOWS_1252
                Err(error2) => {
                    // Both UTF-8 and WINDOWS_1252 failed to decode.
                    // Return the specific EncodingError.
                    Err(MyError::EncodingError(
                        filename.as_ref().to_path_buf(),
                        line_number,
                        error1.to_string(), // Capture UTF-8 error details
                        error2.to_string(), // Capture WINDOWS_1252 error details
                    ))
                }
            }
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
        .skip(1) // Skip the first empty field before the initial delimiter
        .map(|campo| campo.trim().to_string())
        .collect();

    campos.pop(); // Remove the last empty field after the final delimiter

    campos
}

//----------------------------------------------------------------------------//
//                                   Tests                                    //
//----------------------------------------------------------------------------//

/// Run tests with:
/// cargo test -- --show-output lib_tests
#[cfg(test)]
mod lib_tests {
    use super::*;
    use std::{fs, io::Write};
    use tempfile::{tempdir, TempDir};

    // Helper to create a dummy DirEntry for testing
    fn create_dummy_direntry(
        temp_dir: &TempDir,
        filename: &str,
        content: &str,
    ) -> MyResult<DirEntry> {
        let file_path = temp_dir.path().join(filename);
        let mut file = fs::File::create(&file_path)?;
        file.write_all(content.as_bytes())?;

        // walkdir::DirEntry doesn't have a public constructor,
        // so we need to iterate WalkDir to get one.
        WalkDir::new(temp_dir.path())
            .into_iter()
            .flatten()
            .find(|entry| entry.file_name() == filename)
            .ok_or(MyError::TestDummyFileError)
    }

    /// cargo test -- --show-output basic
    #[test]
    fn test_get_map_basic_extraction() -> MyResult<()> {
        let temp_dir = tempdir()?;
        let file_content = r"
|FIELD1|12345678901234567890123456789012345678901234|FIELD2|
|FIELD3|TEXT_WITH_KEY 22222222222222222222222222222222222222222222 END|FIELD4||KEY 11111111111111111111111111111111111111111111|
|FIELD5|ANOTHER 33333333333333333333333333333333333333333333 KEY_HERE|KEY 4444A444444444444444444444444444444444444444|
|FIELD6|NO_KEY_HERE|FIELD7|
        ";
        let entry = create_dummy_direntry(&temp_dir, "PISCOFINS_TEST.txt", file_content)?;

        // --- Start of added code to print file content ---
        let file_path = entry.path(); // Get the path from the DirEntry
        let read_content = fs::read_to_string(file_path)?; // Read the file content
        println!(
            "--- Content of {}: ---\n{}",
            file_path.display(),
            read_content
        );
        // --- End of added code ---

        let result = get_map(&entry)?;

        println!("result: {result:#?}");

        let expected_keys: BTreeSet<String> = BTreeSet::from_iter([
            "11111111111111111111111111111111111111111111".to_string(),
            "12345678901234567890123456789012345678901234".to_string(),
            "22222222222222222222222222222222222222222222".to_string(),
            "33333333333333333333333333333333333333333333".to_string(),
        ]);

        assert_eq!(result, expected_keys);
        Ok(())
    }

    #[test]
    fn test_get_map_with_no_keys() -> MyResult<()> {
        let temp_dir = tempdir()?;
        let file_content = r"
|FIELD1|SOME TEXT|FIELD2|
|FIELD3|NO DIGITS HERE|FIELD4|
        ";
        let entry = create_dummy_direntry(&temp_dir, "PISCOFINS_NOKEYS.txt", file_content)?;

        let result = get_map(&entry)?;
        assert!(result.is_empty());
        Ok(())
    }

    /// cargo test -- --show-output basic
    #[test]
    fn test_get_map_with_duplicate_keys() -> MyResult<()> {
        let temp_dir = tempdir()?;
        let file_content = r"

|FIELD1|KEY3 22222222222222222222222222222222222222222222|
|FIELD2|KEY1 11111111111111111111111111111111111111111111|
|FIELD3|KEY2 11111111111111111111111111111111111111111111|
        ";
        let entry = create_dummy_direntry(&temp_dir, "PISCOFINS_DUPLICATES.txt", file_content)?;

        let result = get_map(&entry)?;

        let expected_keys: BTreeSet<String> = [
            "11111111111111111111111111111111111111111111".to_string(),
            "22222222222222222222222222222222222222222222".to_string(),
        ]
        .into_iter()
        .collect();

        assert_eq!(result, expected_keys);
        assert_eq!(result.len(), 2); // Ensure duplicates are removed
        Ok(())
    }

    /// cargo test -- --show-output 9999
    #[test]
    fn test_get_map_stops_at_9999() -> MyResult<()> {
        let temp_dir = tempdir()?;
        let file_content = r"
|FIELD1|11111111111111111111111111111111111111111111|
|9999|IGNORED_FIELD|22222222222222222222222222222222222222222222|
|FIELD3|33333333333333333333333333333333333333333333|
        ";
        let entry = create_dummy_direntry(&temp_dir, "PISCOFINS_9999.txt", file_content)?;

        let result = get_map(&entry)?;

        println!("result: {result:#?}");

        let expected_keys: BTreeSet<String> =
            ["11111111111111111111111111111111111111111111".to_string()]
                .into_iter()
                .collect();

        println!("expected_keys: {expected_keys:#?}");

        assert_eq!(result, expected_keys);
        assert_eq!(result.len(), 1); // Only the key before 9999 should be captured
        Ok(())
    }

    #[test]
    fn test_get_map_empty_file() -> MyResult<()> {
        let temp_dir = tempdir()?;
        let file_content = r"";
        let entry = create_dummy_direntry(&temp_dir, "PISCOFINS_EMPTY.txt", file_content)?;

        let result = get_map(&entry)?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_get_map_with_different_encodings() -> MyResult<()> {
        // This test is harder to write purely with string literals for WINDOWS_1252
        // because rust string literals are UTF-8.
        // For a true test of get_string_utf8, you'd need to manually create a byte slice
        // that is valid WINDOWS_1252 but invalid UTF-8.
        // Example: b"\xc2" (Â in WINDOWS_1252) is invalid UTF-8 start.
        // For now, we'll rely on the `get_string_utf8`'s internal logic which is tested by
        // converting a valid UTF-8 string to bytes.
        let temp_dir = tempdir()?;
        let file_content_utf8 = r"
|FIELD1|11111111111111111111111111111111111111111111|
|FIELD_ACCENT|áéíóúÁÉÍÓÚ|
        "; // This is UTF-8
        let entry = create_dummy_direntry(&temp_dir, "PISCOFINS_UTF8.txt", file_content_utf8)?;
        let result = get_map(&entry)?;
        assert!(result.contains("11111111111111111111111111111111111111111111"));
        Ok(())
    }
}
