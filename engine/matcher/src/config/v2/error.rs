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
    FileNameError { path: PathBuf },
    DuplicateName { name: String, previous: PathBuf, next: PathBuf },
}

#[derive(Debug)]
pub enum DeploymentError {
    FileIo { path: PathBuf, error: io::Error },
    DirIo { path: PathBuf, error: io::Error },
    // This variant should not be in use. It is just a safeguard, that tornado doesn't crash if
    // any future structures are not serializable.
    Serialization { error: serde_json::Error, data_type: &'static str },
}

// Todo: improve this error in NEPROD-1658
impl From<MatcherConfigError> for MatcherError {
    fn from(value: MatcherConfigError) -> Self {
        MatcherError::ConfigurationError { message: format!("{value:?}") }
    }
}

// Todo: improve this error in NEPROD-1658
impl From<DeploymentError> for MatcherError {
    fn from(value: DeploymentError) -> Self {
        MatcherError::InternalSystemError { message: format!("{value:?}") }
    }
}
