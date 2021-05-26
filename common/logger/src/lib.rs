use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;
use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, EnvFilter};

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
    pub file_output_path: Option<String>,
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
pub fn setup_logger(logger_config: &LoggerConfig) -> Result<Option<WorkerGuard>, LoggerError> {
    let env_filter = EnvFilter::from_str(&logger_config.level).map_err(|err| {
        LoggerError::LoggerConfigurationError {
            message: format!(
                "Cannot parse the logger level: [{}]. err: {}",
                logger_config.level, err
            ),
        }
    })?;

    let (file_subscriber, file_guard) = if let Some(file_output) = &logger_config.file_output_path {
        let (dir, filename) = path_to_dir_and_filename(file_output)?;
        let file_appender = tracing_appender::rolling::never(dir, filename);

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        (Some(Layer::new().with_ansi(false).with_writer(non_blocking)), Some(guard))
    } else {
        (None, None)
    };

    let stdout_subscriber =
        if logger_config.stdout_output { Some(Layer::new().with_ansi(false)) } else { None };

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_subscriber)
        .with(stdout_subscriber);

    set_global_logger(subscriber)?;

    Ok(file_guard)
}

fn path_to_dir_and_filename(full_path: &str) -> Result<(String, String), LoggerError> {
    let full_path = full_path.replace(r#"\"#, "/");
    if let Some(last_separator_index) = full_path.rfind('/') {
        Ok((
            full_path[0..last_separator_index + 1].to_owned(),
            full_path[last_separator_index + 1..full_path.len()].to_owned(),
        ))
    } else {
        Err(LoggerError::LoggerConfigurationError {
            message: format!("Output file format [{}] is wrong", full_path),
        })
    }
}

fn set_global_logger<S>(subscriber: S) -> Result<(), LoggerError>
where
    S: Subscriber + Send + Sync + 'static,
{
    tracing_log::LogTracer::init().map_err(|err| LoggerError::LoggerConfigurationError {
        message: format!("Cannot start the logger LogTracer. err: {}", err),
    })?;
    set_global_default(subscriber).map_err(|err| LoggerError::LoggerConfigurationError {
        message: format!("Cannot start the logger. err: {}", err),
    })
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_split_the_file_path() {
        assert_eq!(
            ("/tmp/hello/".to_owned(), "filename".to_owned()),
            path_to_dir_and_filename("/tmp/hello/filename").unwrap()
        );
        assert_eq!(
            ("/".to_owned(), "log_output.log".to_owned()),
            path_to_dir_and_filename("/log_output.log").unwrap()
        );
        assert_eq!(
            ("/tmp/".to_owned(), "log_output.log".to_owned()),
            path_to_dir_and_filename("/tmp/log_output.log").unwrap()
        );
        assert_eq!(
            ("//tmp///".to_owned(), "log_output.log".to_owned()),
            path_to_dir_and_filename("//tmp///log_output.log").unwrap()
        );
        assert_eq!(
            (
                "/neteye/shared/tornado_rsyslog_collector/log/".to_owned(),
                "tornado_rsyslog_collector.log".to_owned()
            ),
            path_to_dir_and_filename(
                "/neteye/shared/tornado_rsyslog_collector/log/tornado_rsyslog_collector.log"
            )
            .unwrap()
        );
        assert_eq!(
            ("/tmp/hello/".to_owned(), "filename".to_owned()),
            path_to_dir_and_filename(r#"/tmp\hello/filename"#).unwrap()
        );
        assert_eq!(
            ("c:/windows/some/".to_owned(), "filename.txt".to_owned()),
            path_to_dir_and_filename(r#"c:\windows\some\filename.txt"#).unwrap()
        );
    }

    #[test]
    fn split_the_file_path_should_file_if_directory_is_not_present() {
        assert!(path_to_dir_and_filename("filename").is_err());
    }
}
