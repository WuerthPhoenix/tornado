use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

/// Defines the Logger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    // Todo: check if an enum can be used
    /// The Logger level
    /// Valid values: trace, debug, info, warn, error
    pub level: String,

    /// Determines whether the Logger should print to standard output.
    /// Valid values: true, false
    pub stdout_output: bool,

    /// A file path in the file system; if provided, the Logger will append any output to it.
    pub file_output_path: Option<String>,
    // #[structopt(short = "o", long = "value_one", default_value = "10000")]
    // pub module_level: HashMap<String, String>,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        LoggerConfig { level: "info".to_owned(), stdout_output: false, file_output_path: None }
    }
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
    let mut log_dispatcher = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::from_str(&logger_config.level).map_err(|err| {
            LoggerError::LoggerConfigurationError {
                message: format!(
                    "The specified logger level is not valid: [{}]. err: {}",
                    &logger_config.level, err
                ),
            }
        })?);

    /*
    for (module, level) in logger_config.module_level.iter() {
        log_dispatcher =
            log_dispatcher.level_for(module.to_owned(), log::LevelFilter::from_str(level).unwrap())
    }
    */

    log_dispatcher = log_dispatcher
        .level_for("hyper".to_owned(), log::LevelFilter::Warn)
        .level_for("mio".to_owned(), log::LevelFilter::Warn)
        .level_for("rants".to_owned(), log::LevelFilter::Warn)
        .level_for("tokio_io".to_owned(), log::LevelFilter::Warn)
        .level_for("tokio_reactor".to_owned(), log::LevelFilter::Warn)
        .level_for("tokio_tcp".to_owned(), log::LevelFilter::Warn)
        .level_for("tokio_uds".to_owned(), log::LevelFilter::Warn)
        .level_for("tokio_util".to_owned(), log::LevelFilter::Warn);

    if logger_config.stdout_output {
        log_dispatcher = log_dispatcher.chain(std::io::stdout());
    }

    if let Some(path) = &logger_config.file_output_path {
        log_dispatcher = log_dispatcher.chain(fern::log_file(&path)?)
    }

    log_dispatcher.apply()?;

    Ok(())
}
