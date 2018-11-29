extern crate chrono;
extern crate failure;
extern crate fern;
extern crate log;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate structopt;

use std::collections::HashMap;
use std::str::FromStr;
use structopt::StructOpt;

/// The logger configuration.
#[derive(Debug, Clone, StructOpt)]
pub struct LoggerConfig {

    // Todo: check if an enum can be used

    /// The logger level
    /// Valid values: trace, debug, info, warn, error
    #[structopt(long = "logger-level", default_value = "warn")]
    pub level: String,

    /// Whether the logger should print on the standard output.
    /// Valid values: true, false
    #[structopt(long = "logger-stdout")]
    pub stdout_output: bool,

    /// A file path on the file system.
    /// If provided, the logger will append any output to it.
    #[structopt(long = "logger-file-path")]
    pub file_output_path: Option<String>,

    // #[structopt(short = "o", long = "value_one", default_value = "10000")]
    // pub module_level: HashMap<String, String>,

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
        }).level(log::LevelFilter::from_str(&logger_config.level).unwrap());

    /*
    for (module, level) in logger_config.module_level.iter() {
        log_dispatcher =
            log_dispatcher.level_for(module.to_owned(), log::LevelFilter::from_str(level).unwrap())
    }
    */

    if logger_config.stdout_output {
        log_dispatcher = log_dispatcher.chain(std::io::stdout());
    }

    match &logger_config.file_output_path {
        Some(path) => log_dispatcher = log_dispatcher.chain(fern::log_file(&path)?),
        _=> {}
    }

    log_dispatcher.apply()?;

    Ok(())
}
