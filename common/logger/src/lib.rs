use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

/// Defines the Logger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    /// Sets the logger [`EnvFilter`].
    /// Valid values: trace, debug, info, warn, error
    /// Example of a valid filter: "warn,my_crate=info,my_crate::my_mod=debug,[my_span]=trace"
    pub level: String,

    /// Determines whether the Logger should print to standard output.
    /// Valid values: true, false
    pub stdout_output: bool,

    // A file path in the file system; if provided, the Logger will append any output to it;
    // otherwise, it will log on the stdout.
    // pub file_output_path: Option<String>,

}

#[derive(Error, Debug)]
pub enum LoggerError {
    #[error("LoggerConfigurationError: [{message}]")]
    LoggerConfigurationError { message: String },
}

impl From<log::SetLoggerError> for LoggerError {
    fn from(error: log::SetLoggerError) -> Self {
        LoggerError::LoggerConfigurationError { message: format!("{}", error) }
    }
}

impl From<std::io::Error> for LoggerError {
    fn from(error: std::io::Error) -> Self {
        LoggerError::LoggerConfigurationError { message: format!("{}", error) }
    }
}

/// Configures the underlying logger implementation and activates it.
pub fn setup_logger(logger_config: &LoggerConfig) -> Result<(), LoggerError> {
    if logger_config.stdout_output {
        let env_filter = EnvFilter::from_str(&logger_config.level).map_err(|err| {
            LoggerError::LoggerConfigurationError {
                message: format!(
                    "Cannot parse the env_filter: [{}]. err: {}",
                    logger_config.level, err
                ),
            }
        })?;

        FmtSubscriber::builder()
            .with_env_filter(env_filter)
            .try_init()
            .map_err(|err| LoggerError::LoggerConfigurationError {
                message: format!("Cannot start the stdout_output logger. err: {}", err),
            })?;
    }

    Ok(())
}
