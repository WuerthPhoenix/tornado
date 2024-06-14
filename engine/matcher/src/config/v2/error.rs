use crate::config::v2::ConfigType;
use crate::error::MatcherError;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum MatcherConfigError {
    DirIoError { path: PathBuf, error: io::Error },
    UnexpectedFile { path: PathBuf, config_type: ConfigType },
    UnknownNodeDir { path: PathBuf },
    FileNotFound { path: PathBuf },
    FileIoError { path: PathBuf, error: io::Error },
    DeserializationError { file: PathBuf, error: DeserializationError },
    DuplicateName { name: String, previous: PathBuf, next: PathBuf },
}

#[derive(Debug)]
pub enum DeserializationError {
    UnknownField {
        path: String,
        field: String,
    },
    MissingField {
        path: String,
        field: String,
    },
    InvalidField {
        path: String,
        found: String,
        found_type: String,
        expected: String,
        expected_type: String,
    },
    TypeError {
        path: String,
        expected_type: String,
        actual_type: String,
    },
    FormatError {
        line: usize,
        column: usize,
    },
    // This variant should not be in use, however we need it to satisfy the compiler
    // and to avoid future breaking changes
    GenericError {
        error: String,
    },
}

// Todo: improve this error in NEPROD-1658
impl From<MatcherConfigError> for MatcherError {
    fn from(value: MatcherConfigError) -> Self {
        MatcherError::ConfigurationError { message: format!("{value:?}") }
    }
}
