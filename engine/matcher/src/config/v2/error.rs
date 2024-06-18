use crate::config::v2::ConfigType;
use crate::error::MatcherError;
use std::error::Error;
use std::fmt::{Display, Formatter};
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

impl Display for MatcherConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MatcherConfigError::DirIoError { error, path } => f.write_fmt(format_args!(
                "IO Error while reading the directory {}: {}",
                path.display(),
                error
            )),
            MatcherConfigError::UnexpectedFile { path, config_type } => f.write_fmt(format_args!(
                "Encountered unexpected file {} while parsing node of type {}",
                path.display(),
                config_type
            )),
            MatcherConfigError::UnknownNodeDir { path } => f.write_fmt(format_args!(
                "Cannot parse node in directory {}. It does not contain any config file.",
                path.display()
            )),
            MatcherConfigError::FileNotFound { path } => f.write_fmt(format_args!(
                "Expected to find file {}, but it was not present.",
                path.display()
            )),
            MatcherConfigError::FileIoError { path, error } => f.write_fmt(format_args!(
                "Error while reading config file {}: {}",
                path.display(),
                error
            )),
            MatcherConfigError::FileNameError { path } => f.write_fmt(format_args!(
                "Could not read a file with filename {}, because it is not utf-8.",
                path.display()
            )),
            MatcherConfigError::DeserializationError { file, object_path, error,  } => f.write_fmt(format_args!(
                "Could not deserialize config file {}. Field {} contains erroneous data: {}",
                file.display(),
                object_path,
                error
            )),
            MatcherConfigError::DuplicateName { name, previous, next } => f.write_fmt(format_args!(
                "Duplicate name {} in config detected. The node was first declared here: {} and then redelared here: {}",
                name,
                previous.display(),
                next.display()
            )),
            MatcherConfigError::FormatError { file, error } => f.write_fmt(format_args!(
                "Could not deserialize config file {}, as it is not a valid json file: {}",
                file.display(),
                error
            )),
        }
    }
}

impl Error for MatcherConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MatcherConfigError::DirIoError { error, .. } => Some(error as &dyn Error),
            MatcherConfigError::FileIoError { error, .. } => Some(error as &dyn Error),
            MatcherConfigError::DeserializationError { error, .. } => Some(error as &dyn Error),
            MatcherConfigError::FormatError { error, .. } => Some(error as &dyn Error),
            MatcherConfigError::UnexpectedFile { .. } => None,
            MatcherConfigError::UnknownNodeDir { .. } => None,
            MatcherConfigError::FileNotFound { .. } => None,
            MatcherConfigError::FileNameError { .. } => None,
            MatcherConfigError::DuplicateName { .. } => None,
        }
    }
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
