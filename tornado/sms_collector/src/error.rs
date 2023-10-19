#![allow(clippy::enum_variant_names)]

use crate::SmsFile;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SmsCollectorConfigError {
    #[error("Could not parse the commandline arguments: {0}")]
    ArgumentParseError(#[from] clap::Error),
    #[error("Could not load config file for the collector: {0}")]
    ConfigError(#[from] config_rs::ConfigError),
    #[error("Could not instantiate logger: {0}")]
    LoggerError(#[from] tornado_common_logger::LoggerError),
}

#[derive(Error, Debug)]
pub enum SmsCollectorError {
    #[error("Could not parse sms file correctly: {error}")]
    SmsParseError { error: SmsParseError },
    #[error("Could not find or open sms file {sms_file}: {error} ")]
    SmsFileAccessError { sms_file: SmsFile, error: io::Error },
    #[error("Could not send data to nats, {error} - The sms will be copied to {failed_sms_file}")]
    TornadoConnectionError { error: io::Error, failed_sms_file: PathBuf },
}

#[derive(Error, Debug)]
pub enum SmsParseError {
    #[error("{0}")]
    FormatError(String),
    #[error("{0}")]
    ContentError(serde_json::Error),
}
