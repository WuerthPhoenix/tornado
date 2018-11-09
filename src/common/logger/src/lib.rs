extern crate chrono;
extern crate failure;
extern crate fern;
extern crate log;
#[macro_use]
extern crate failure_derive;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;
use std::str::FromStr;

/// The logger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    pub root_level: String,
    pub output_system_enabled: bool,
    pub output_file_enabled: bool,
    pub output_file_name: String,
    pub module_level: HashMap<String, String>,
}

#[derive(Fail, Debug)]
pub enum LoggerError {
    #[fail(display = "LoggerConfigurationError: [{}]", message)]
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

/// It configures the underlying logger implementation and activate it.
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
        }).level(log::LevelFilter::from_str(&logger_config.root_level).unwrap());

    for (module, level) in logger_config.module_level.iter() {
        log_dispatcher =
            log_dispatcher.level_for(module.to_owned(), log::LevelFilter::from_str(level).unwrap())
    }

    if logger_config.output_system_enabled {
        log_dispatcher = log_dispatcher.chain(std::io::stdout());
    }

    if logger_config.output_file_enabled {
        log_dispatcher = log_dispatcher.chain(fern::log_file(&logger_config.output_file_name)?);
    }

    log_dispatcher.apply()?;

    Ok(())
}
