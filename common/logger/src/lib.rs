use crate::elastic_apm::{get_current_service_name, ApmTracingConfig};
use arc_swap::ArcSwap;
use opentelemetry::sdk::propagation::TraceContextPropagator;
use opentelemetry::sdk::trace;
use opentelemetry::sdk::trace::{Sampler, Tracer};
use opentelemetry::sdk::Resource;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::{ExportConfig, Protocol, WithExportConfig};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tonic::metadata::MetadataMap;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::{filter_fn, Targets};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, layer::SubscriberExt, Layer, Registry};

pub mod elastic_apm;

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

    pub tracing_elastic_apm: ApmTracingConfig,
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
    apm_enabled: Arc<AtomicBool>,

    reload_handle: tracing_subscriber::reload::Handle<Targets, Registry>,
}

impl LogWorkerGuard {
    pub fn new(
        file_guard: Option<WorkerGuard>,
        stdout_guard: Option<WorkerGuard>,
        config: Arc<LoggerConfig>,
        stdout_enabled: Arc<AtomicBool>,
        apm_enabled: Arc<AtomicBool>,
        reload_handle: tracing_subscriber::reload::Handle<Targets, Registry>,
    ) -> Self {
        let logger_level = ArcSwap::from(Arc::new(config.level.clone()));
        Self {
            file_guard,
            stdout_guard,
            config,
            logger_level,
            stdout_enabled,
            apm_enabled,
            reload_handle,
        }
    }

    pub fn level(&self) -> String {
        self.logger_level.load().as_ref().to_owned()
    }

    /// Reloads the logger global filter
    pub fn set_level<S: Into<String>>(&self, env_filter_str: S) -> Result<(), LoggerError> {
        let filter = env_filter_str.into();
        let env_filter =
            Targets::from_str(&filter).map_err(|err| LoggerError::LoggerConfigurationError {
                message: format!("Cannot parse the logger level: [{}]. err: {:?}", filter, err),
            })?;
        self.reload_handle.reload(env_filter).map_err(|err| {
            LoggerError::LoggerConfigurationError {
                message: format!("Cannot reload the logger configuration. err: {:?}", err),
            }
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
        self.apm_enabled.load(Ordering::Relaxed)
    }

    pub fn set_apm_enabled(&self, enabled: bool) {
        self.apm_enabled.store(enabled, Ordering::Relaxed)
    }
}

/// Configures the underlying logger implementation and activates it.
pub fn setup_logger(logger_config: LoggerConfig) -> Result<LogWorkerGuard, LoggerError> {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let config_logger_level = Arc::new(logger_config.level.to_owned());
    let logger_level = ArcSwap::new(config_logger_level);
    let env_filter = Targets::from_str(&logger_config.level).map_err(|err| {
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

        (Some(fmt::Layer::new().with_writer(non_blocking)), Some(guard))
    } else {
        (None, None)
    };

    let stdout_enabled = Arc::new(AtomicBool::new(logger_config.stdout_output));

    let (stdout_subscriber, stdout_guard) = {
        let (non_blocking, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());

        let stdout_enabled = stdout_enabled.clone();

        (
            fmt::Layer::new()
                .with_writer(non_blocking)
                .with_filter(filter_fn(move |_meta| stdout_enabled.load(Ordering::Relaxed))),
            Some(stdout_guard),
        )
    };

    let (apm_layer, apm_enabled) = {
        let tracer = get_opentelemetry_tracer(&logger_config.tracing_elastic_apm)?;
        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

        let enabled = Arc::new(AtomicBool::new(logger_config.tracing_elastic_apm.apm_output));
        let enabled_clone = enabled.clone();

        (
            telemetry.with_filter(filter_fn(move |_meta| enabled.load(Ordering::Relaxed))),
            enabled_clone,
        )
    };

    tracing_subscriber::registry()
        .with(reloadable_env_filter)
        .with(file_subscriber)
        .with(stdout_subscriber)
        .with(apm_layer)
        .try_init()
        .map_err(|err| LoggerError::LoggerConfigurationError {
            message: format!("Cannot start the logger. err: {:?}", err),
        })?;

    Ok(LogWorkerGuard {
        file_guard,
        stdout_guard,
        config: logger_config.into(),
        logger_level,
        reload_handle: reloadable_env_filter_handle,
        stdout_enabled,
        apm_enabled,
    })
}

fn get_opentelemetry_tracer(apm_tracing_config: &ApmTracingConfig) -> Result<Tracer, LoggerError> {
    let mut tonic_metadata = MetadataMap::new();
    if let Some(apm_server_api_credentials) = &apm_tracing_config.apm_server_api_credentials {
        tonic_metadata.insert(
            "authorization",
            apm_server_api_credentials.to_authorization_header_value().parse()
                .map_err(|err| LoggerError::LoggerRuntimeError {
                    message: format!("Logger - Error while constructing the authorization header for tonic client. Error: {}", err)
                })?,
        );
    };

    let export_config = ExportConfig {
        endpoint: apm_tracing_config.apm_server_url.clone(),
        protocol: Protocol::Grpc,
        timeout: Duration::from_secs(10),
    };
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_export_config(export_config)
                .with_metadata(tonic_metadata),
        )
        .with_trace_config(trace::config().with_sampler(Sampler::AlwaysOn).with_resource(
            Resource::new(vec![KeyValue::new("service.name", get_current_service_name()?)]),
        ))
        .install_batch(opentelemetry::runtime::Tokio)
        .map_err(|err| LoggerError::LoggerRuntimeError {
            message: format!(
                "Logger - Error while installing the OpenTelemetry Tracer. Error: {:?}",
                err
            ),
        })
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::elastic_apm::ApmServerApiCredentials;

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
            tracing_elastic_apm: ApmTracingConfig::default(),
        };
        let env_filter = Targets::from_str(&config.level).unwrap();
        let logger_level = ArcSwap::new(Arc::new(config.level.clone()));

        let guard = LogWorkerGuard {
            apm_enabled: AtomicBool::new(false).into(),
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
            tracing_elastic_apm: ApmTracingConfig::default(),
        };
        let env_filter = Targets::from_str(&config.level).unwrap();
        let logger_level = ArcSwap::new(Arc::new(config.level.clone()));

        let guard = LogWorkerGuard {
            apm_enabled: AtomicBool::new(true).into(),
            stdout_enabled: AtomicBool::new(true).into(),
            config: Arc::new(config.clone()),
            logger_level,
            file_guard: None,
            stdout_guard: None,
            reload_handle: tracing_subscriber::reload::Layer::new(env_filter).1,
        };

        // Act
        guard.set_apm_enabled(true);
        assert!(guard.apm_enabled());

        guard.set_apm_enabled(false);
        assert!(!guard.apm_enabled());
    }

    #[test]
    fn log_worker_guard_should_set_logger_level() {
        // Arrange
        let config = LoggerConfig {
            level: "info".to_owned(),
            stdout_output: true,
            file_output_path: None,
            tracing_elastic_apm: ApmTracingConfig::default(),
        };
        let env_filter = Targets::from_str(&config.level).unwrap();
        let logger_level = ArcSwap::new(Arc::new(config.level.clone()));

        let (reloadable_env_filter, reloadable_env_filter_handle) =
            tracing_subscriber::reload::Layer::new(env_filter);

        let _subscriber = tracing_subscriber::registry().with(reloadable_env_filter);

        let guard = LogWorkerGuard {
            apm_enabled: AtomicBool::new(false).into(),
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
            tracing_elastic_apm: ApmTracingConfig::default(),
        };
        let env_filter = Targets::from_str(&config.level).unwrap();
        let logger_level = ArcSwap::new(Arc::new(config.level.clone()));

        let (reloadable_env_filter, reloadable_env_filter_handle) =
            tracing_subscriber::reload::Layer::new(env_filter);

        let _subscriber = tracing_subscriber::registry().with(reloadable_env_filter);

        let guard = LogWorkerGuard {
            apm_enabled: AtomicBool::new(false).into(),
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

    #[tokio::test]
    async fn should_get_opentelemetry_tracer() {
        let tracing_config = ApmTracingConfig {
            apm_output: true,
            apm_server_url: "apm.example.com".to_string(),
            apm_server_api_credentials: Some(ApmServerApiCredentials {
                id: "myid".to_string(),
                key: "mykey".to_string(),
            }),
        };
        let tracer = get_opentelemetry_tracer(&tracing_config);
        assert!(tracer.is_ok());
    }
}
