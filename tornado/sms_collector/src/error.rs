use crate::SmsFile;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SmsCollectorError {
    #[error("{0}")]
    ArgumentParseError(#[from] clap::Error),
    #[error("Could not load config file for the collector: {error}")]
    ConfigError {
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
        event: String,
        sms_file: SmsFile,
    },
    #[error("Could not parse sms file correctly: {error}")]
    SmsParseError { error: SmsParseError, sms_file: SmsFile },
    #[error("Ignoring event \"{event}\"")]
    IgnoreAction { event: String, sms_file: SmsFile },
    #[error("Could not find or open sms file {sms_file}: {error} ")]
    SmsFileAccessError { sms_file: SmsFile, error: io::Error },
    #[error("Could not forward sms contents to nats. The sms will be copied to {failed_sms_file}")]
    TornadoConnectionError { error: io::Error, sms_file: SmsFile, failed_sms_file: PathBuf },
}

impl SmsCollectorError {
    pub fn sms_file(&self) -> Option<&SmsFile> {
        match self {
            SmsCollectorError::ArgumentParseError(_) => None,
            SmsCollectorError::IgnoreAction { sms_file, .. } => Some(sms_file),
            SmsCollectorError::SmsFileAccessError { sms_file, .. } => Some(sms_file),
            SmsCollectorError::TornadoConnectionError { sms_file, .. } => Some(sms_file),
            SmsCollectorError::ConfigError { sms_file, .. } => Some(sms_file),
            SmsCollectorError::SmsParseError { sms_file, .. } => Some(sms_file),
        }
    }
}

#[derive(Error, Debug)]
pub enum SmsParseError {
    #[error("{0}")]
    FormatError(String),
    #[error("{0}")]
    ContentError(serde_json::Error),
}
