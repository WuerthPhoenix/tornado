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
    DeserializationError { file: PathBuf, object_path: String, error: serde_json::Error },
    FormatError { file: PathBuf, error: serde_json::Error },
    DuplicateName { name: String, previous: PathBuf, next: PathBuf },
}

// Todo: improve this error in NEPROD-1658
impl From<MatcherConfigError> for MatcherError {
    fn from(value: MatcherConfigError) -> Self {
        MatcherError::ConfigurationError { message: format!("{value:?}") }
    }
}
