use crate::elastic_apm::{get_current_service_name, ApmTracingConfig};
use arc_swap::ArcSwap;
use crate::filter::FilteredLayer;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_elastic_apm::config::Authorization;
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, EnvFilter, Registry};

pub mod elastic_apm;

mod filter;

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

    pub tracing_elastic_apm: Option<ApmTracingConfig>,
}

#[derive(Error, Debug)]
pub enum LoggerError {
    #[error("LoggerConfigurationError: [{message}]")]
    LoggerConfigurationError { message: String },
    #[error("LoggerRuntimeError: [{message}]")]
    LoggerRuntimeError { message: String },
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

pub struct LogWorkerGuard {
    #[allow(dead_code)]
    file_guard: Option<WorkerGuard>,
    #[allow(dead_code)]
    stdout_guard: Option<WorkerGuard>,

    // The logger original configuration
    config: Arc<LoggerConfig>,

    logger_level: ArcSwap<String>,
    stdout_enabled: Arc<AtomicBool>,
    apm_enabled: Option<Arc<AtomicBool>>,

    reload_handle: tracing_subscriber::reload::Handle<EnvFilter, Registry>,
}

impl LogWorkerGuard {
    pub fn new(
        file_guard: Option<WorkerGuard>,
        stdout_guard: Option<WorkerGuard>,
        config: Arc<LoggerConfig>,
        stdout_enabled: Arc<AtomicBool>,
        apm_enabled: Option<Arc<AtomicBool>>,
        reload_handle: tracing_subscriber::reload::Handle<EnvFilter, Registry>,
    ) -> Self {
        let logger_level = ArcSwap::from(Arc::new(config.level.clone()));
        Self { file_guard, stdout_guard, config, logger_level, stdout_enabled, apm_enabled, reload_handle }
    }

    pub fn level(&self) -> String {
        self.logger_level.load().as_ref().to_owned()
    }

    /// Reloads the logger global filter
    pub fn set_level<S: Into<String>>(&self, env_filter_str: S) -> Result<(), LoggerError> {
        let filter = env_filter_str.into();
        let env_filter = EnvFilter::from_str(&filter).map_err(|err| {
            LoggerError::LoggerConfigurationError {
                message: format!(
                    "Cannot parse the logger level: [{}]. err: {:?}",
                    filter, err
                ),
            }
        })?;
        self.reload_handle.reload(env_filter).map_err(|err| LoggerError::LoggerConfigurationError {
            message: format!("Cannot reload the logger configuration. err: {:?}", err),
        })?;

        self.logger_level.store(Arc::new(filter));

        Ok(())
    }

    /// Reset the logger global filter to the original value from the configuration
    pub fn reset_level(&self) -> Result<(), LoggerError> {
        self.set_level(&self.config.level)
    }

    pub fn stdout_enabled(&self) -> bool {
        self.stdout_enabled.load(Ordering::Relaxed)
    }

    pub fn set_stdout_enabled(&self, enabled: bool) {
        self.stdout_enabled.store(enabled, Ordering::Relaxed)
    }

    pub fn apm_enabled(&self) -> bool {
        self.apm_enabled.as_ref().map(|val| val.load(Ordering::Relaxed)).unwrap_or(false)
    }

    pub fn set_apm_enabled(&self, enabled: bool) -> Result<(), LoggerError> {
        self.apm_enabled
            .as_ref()
            .ok_or_else(|| LoggerError::LoggerConfigurationError {
                message: format!(
                    "Cannot enable/disable the apm logger because it is not configured."
                ),
            })
            .map(|apm_enabled| apm_enabled.store(enabled, Ordering::Relaxed))
    }
}

/// Configures the underlying logger implementation and activates it.
pub fn setup_logger(
    logger_config: LoggerConfig,
) -> Result<LogWorkerGuard, LoggerError> {
    let config_logger_level = Arc::new(logger_config.level.to_owned());
    let logger_level = ArcSwap::new(config_logger_level.clone());
    let env_filter = EnvFilter::from_str(&logger_config.level).map_err(|err| {
        LoggerError::LoggerConfigurationError {
            message: format!(
                "Cannot parse the logger level: [{}]. err: {:?}",
                logger_config.level, err
            ),
        }
    })?;

    let (reloadable_env_filter, reloadable_env_filter_handle) =
        tracing_subscriber::reload::Layer::new(env_filter);

    let (file_subscriber, file_guard) = if let Some(file_output) = &logger_config.file_output_path {
        let (dir, filename) = path_to_dir_and_filename(file_output)?;
        let file_appender = tracing_appender::rolling::never(dir, filename);

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        (Some(Layer::new().with_ansi(false).with_writer(non_blocking)), Some(guard))
    } else {
        (None, None)
    };

    let stdout_enabled = Arc::new(AtomicBool::new(logger_config.stdout_output));

    let (stdout_subscriber, stdout_guard) = {
        let (non_blocking, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());

        let stdout_enabled = stdout_enabled.clone();

        (
            FilteredLayer::new(
                Layer::new().with_ansi(false).with_writer(non_blocking),
                move |_metadata, _ctx| stdout_enabled.load(Ordering::Relaxed),
            ),
            Some(stdout_guard),
        )
    };

    let mut apm_enabled = None;

    let apm_layer = if let Some(apm_tracing_config) = logger_config.tracing_elastic_apm.clone() {
        let mut apm_config =
            tracing_elastic_apm::config::Config::new(apm_tracing_config.apm_server_url.clone());
        apm_config = if let Some(apm_server_api_credentials) =
            apm_tracing_config.apm_server_api_credentials
        {
            apm_config.with_authorization(Authorization::ApiKey(apm_server_api_credentials.into()))
        } else {
            apm_config
        };
        let apm_layer = tracing_elastic_apm::new_layer(get_current_service_name()?, apm_config)
            .map_err(|err| LoggerError::LoggerConfigurationError {
                message: format!(
                    "Could not create APM tracing layer for the logger. Err: {:?}",
                    err
                ),
            })?;
        let enabled = Arc::new(AtomicBool::new(true));
        apm_enabled = Some(enabled.clone());

        Some(FilteredLayer::new(apm_layer, move |_metadata, _ctx| enabled.load(Ordering::Relaxed)))
    } else {
        None
    };

    let subscriber = tracing_subscriber::registry()
        .with(reloadable_env_filter)
        .with(file_subscriber)
        .with(stdout_subscriber)
        .with(apm_layer);

    set_global_logger(subscriber)?;

    Ok(LogWorkerGuard { file_guard, stdout_guard, config: logger_config.into(), logger_level, reload_handle: reloadable_env_filter_handle, stdout_enabled, apm_enabled })
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
        message: format!("Cannot start the logger LogTracer. err: {:?}", err),
    })?;
    set_global_default(subscriber).map_err(|err| LoggerError::LoggerConfigurationError {
        message: format!("Cannot start the logger. err: {:?}", err),
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
    fn log_worker_guard_should_set_stdoud_enabled() {
        // Arrange
        let config = LoggerConfig {
            level: "info".to_owned(),
            stdout_output: true,
            file_output_path: None,
            tracing_elastic_apm: None
        };
        let env_filter = EnvFilter::from_str(&config.level).unwrap();
        let logger_level = ArcSwap::new(Arc::new(config.level.clone()));

        let guard = LogWorkerGuard {
            apm_enabled: None,
            stdout_enabled: AtomicBool::new(true).into(),
            file_guard: None,
            config: Arc::new(config.clone()),
            logger_level,
            stdout_guard: None,
            reload_handle: tracing_subscriber::reload::Layer::new(env_filter).1,
        };

        // Act
        guard.set_stdout_enabled(true);
        assert!(guard.stdout_enabled());

        guard.set_stdout_enabled(false);
        assert!(!guard.stdout_enabled());
    }

    #[test]
    fn log_worker_guard_should_set_apm_enabled() {
        // Arrange
        let config = LoggerConfig {
            level: "info".to_owned(),
            stdout_output: true,
            file_output_path: None,
            tracing_elastic_apm: None
        };
        let env_filter = EnvFilter::from_str(&config.level).unwrap();
        let logger_level = ArcSwap::new(Arc::new(config.level.clone()));

        let guard = LogWorkerGuard {
            apm_enabled: Some(AtomicBool::new(true).into()),
            stdout_enabled: AtomicBool::new(true).into(),
            config: Arc::new(config.clone()),
            logger_level,
            file_guard: None,
            stdout_guard: None,
            reload_handle: tracing_subscriber::reload::Layer::new(env_filter).1,
        };

        // Act
        guard.set_apm_enabled(true).unwrap();
        assert!(guard.apm_enabled());

        guard.set_apm_enabled(false).unwrap();
        assert!(!guard.apm_enabled());
    }

    #[test]
    fn log_worker_guard_should_fail_if_set_apm_enabled_without_config() {
        // Arrange
        let config = LoggerConfig {
            level: "info".to_owned(),
            stdout_output: true,
            file_output_path: None,
            tracing_elastic_apm: None
        };
        let env_filter = EnvFilter::from_str(&config.level).unwrap();
        let logger_level = ArcSwap::new(Arc::new(config.level.clone()));

        let guard = LogWorkerGuard {
            apm_enabled: None,
            stdout_enabled: AtomicBool::new(true).into(),
            config: Arc::new(config.clone()),
            logger_level,
            file_guard: None,
            stdout_guard: None,
            reload_handle: tracing_subscriber::reload::Layer::new(env_filter).1,
        };

        // Act
        assert!(guard.set_apm_enabled(true).is_err());
    }

    #[test]
    fn log_worker_guard_should_set_logger_level() {
        // Arrange
        let config = LoggerConfig {
            level: "info".to_owned(),
            stdout_output: true,
            file_output_path: None,
            tracing_elastic_apm: None
        };
        let env_filter = EnvFilter::from_str(&config.level).unwrap();
        let logger_level = ArcSwap::new(Arc::new(config.level.clone()));

        let (reloadable_env_filter, reloadable_env_filter_handle) =
            tracing_subscriber::reload::Layer::new(env_filter);

        let _subscriber = tracing_subscriber::registry()
        .with(reloadable_env_filter);

        let guard = LogWorkerGuard {
            apm_enabled: None,
            stdout_enabled: AtomicBool::new(true).into(),
            file_guard: None,
            config: Arc::new(config.clone()),
            logger_level,
            stdout_guard: None,
            reload_handle: reloadable_env_filter_handle,
        };

        // Act
        assert_eq!("info", &guard.level());

        assert!(guard.set_level("debug").is_ok());
        assert_eq!("debug", &guard.level());

        assert!(guard.set_level("NOT_VALID_FILTER,,==::!$&%$££$%").is_err());
        assert_eq!("debug", &guard.level());
    }

    #[test]
    fn log_worker_guard_should_reset_logger_level_to_original_config() {
        // Arrange
        let config = LoggerConfig {
            level: "warn,tornado=debug".to_owned(),
            stdout_output: true,
            file_output_path: None,
            tracing_elastic_apm: None
        };
        let env_filter = EnvFilter::from_str(&config.level).unwrap();
        let logger_level = ArcSwap::new(Arc::new(config.level.clone()));

        let (reloadable_env_filter, reloadable_env_filter_handle) =
            tracing_subscriber::reload::Layer::new(env_filter);

        let _subscriber = tracing_subscriber::registry()
            .with(reloadable_env_filter);

        let guard = LogWorkerGuard {
            apm_enabled: None,
            stdout_enabled: AtomicBool::new(true).into(),
            file_guard: None,
            config: Arc::new(config.clone()),
            logger_level,
            stdout_guard: None,
            reload_handle: reloadable_env_filter_handle,
        };

        // Act
        assert!(guard.set_level("debug").is_ok());
        assert_eq!("debug", &guard.level());

        assert!(guard.reset_level().is_ok());
        assert_eq!(config.level.as_str(), &guard.level());
    }

    #[test]
    fn split_the_file_path_should_file_if_directory_is_not_present() {
        assert!(path_to_dir_and_filename("filename").is_err());
    }
}
