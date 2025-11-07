use std::{error::Error, io, path::PathBuf};
use thiserror::Error;

/// A specialized `Result` type for this application's operations.
///
/// This type is used to simplify error handling, allowing functions
/// to return `MyResult<T>` instead of `Result<T, MyError>`.
pub type MyResult<T> = Result<T, MyError>;

/// Custom error types for the application.
///
/// This enum centralizes all possible errors that can occur during
/// file processing, key extraction, and argument validation.
#[derive(Debug, Error)]
pub enum MyError {
    /// Error when text cannot be decoded from expected encodings (UTF-8, WINDOWS-1252).
    #[error("Failed to decode text from file '{0}' on line {1}. UTF-8 error: {2}, WINDOWS-1252 error: {3}")]
    EncodingError(PathBuf, usize, String, String), // Path, line number, UTF-8 error, WINDOWS-1252 error

    /// Custom error variant to signal that the "9999" end-of-file marker was reached.
    /// This is treated as a normal, non-error termination condition for the processing loop.
    #[error("End-of-file marker '9999' reached in file '{0}' at line {1}, stopping processing.")]
    EofMarkerReached(PathBuf, usize),

    /// Error encountered when failing to open a file for reading.
    #[error("Could not open file '{0}' for reading: {1}")]
    FileReadError(PathBuf, io::Error),

    /// Error encountered when failing to open a file for writing.
    #[error("Could not open file '{0}' for writing: {1}")]
    FileWriteError(PathBuf, io::Error),

    /// Error when a specified path does not exist.
    #[error("Path '{0}' not found.")]
    PathNotFound(PathBuf),

    /// Error when a specified path is not a directory.
    #[error("Path '{0}' is not a directory.")]
    NotADirectory(PathBuf),

    /// Error when attempting to write to a read-only directory.
    #[error("Directory '{0}' is read-only. No write permission.")]
    ReadOnlyDirectory(PathBuf),

    /// Error during directory traversal or file listing.
    #[error("Error listing files in '{0}': {1}")]
    FileListError(PathBuf, io::Error),

    /// Error that occurred during the processing of a specific EFD file.
    /// The inner error provides more details about the failure.
    #[error("Failed to process EFD file '{0}': {1}")]
    FileProcessingError(PathBuf, Box<MyError>),

    /// Error finding a dummy file created in tests (specific to test helper).
    #[error("Test helper error: Could not find created dummy file.")]
    TestDummyFileError,

    /// General I/O error, often converted from `std::io::Error`.
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// Error related to regex operations (e.g., malformed regex).
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    /// Error from `walkdir` crate when traversing directories.
    #[error("Walkdir error: {0}")]
    WalkdirError(#[from] walkdir::Error),

    /// Um catch-all para outros erros menos específicos não cobertos por variantes específicas.
    #[error("Outro erro subjacente: {0}")]
    Other(String), // Wrapped boxed error
}

// Implement From<String> para MyError, caso precise converter strings genéricas em erros.
impl From<String> for MyError {
    fn from(err: String) -> Self {
        MyError::Other(err)
    }
}

// Implementa a conversão de Box<dyn Error + Send + Sync> para MyError
impl From<Box<dyn Error + Send + Sync>> for MyError {
    fn from(err: Box<dyn Error + Send + Sync>) -> Self {
        MyError::Other(err.to_string())
    }
}
