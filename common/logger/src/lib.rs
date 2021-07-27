use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;
use thiserror::Error;
use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_elastic_apm::config::{ApiKey, Authorization};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, EnvFilter, Registry};

const DEFAULT_APM_SERVER_CREDENTIALS_FILENAME: &str = "apm_server_api_credentials.json";

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

    #[serde(default)]
    pub apm_tracing: ApmTracingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApmTracingConfig {
    // The url of the Elastic APM server; if provided, traces will be sent to this server;
    // if not provided traces will not be sent.
    pub apm_server_url: Option<String>,

    // The path of file containing credentials for calling the APM server APIs;
    pub apm_server_credentials_filepath: Option<String>,
}

impl Default for ApmTracingConfig {
    fn default() -> Self {
        Self { apm_server_url: None, apm_server_credentials_filepath: None }
    }
}

impl ApmTracingConfig {
    fn read_api_credentials(&self, config_dir: &str) -> Result<ApiCredentials, LoggerError> {
        let apm_server_credentials_filepath = if let Some(apm_server_credentials_filepath) =
            self.apm_server_credentials_filepath.clone()
        {
            apm_server_credentials_filepath
        } else {
            format!("{}/{}", config_dir, self::DEFAULT_APM_SERVER_CREDENTIALS_FILENAME)
        };
        let apm_server_credentials_file = File::open(&apm_server_credentials_filepath)?;
        let apm_server_credentials_reader = BufReader::new(apm_server_credentials_file);

        serde_json::from_reader(apm_server_credentials_reader).map_err(|err| {
            LoggerError::LoggerConfigurationError {
                message: format!(
                    "Failed to read APM server Api Key from file {}. Error: {:?}",
                    &apm_server_credentials_filepath, err
                ),
            }
        })
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct ApiCredentials {
    id: String,
    key: String,
}

fn get_current_service_name() -> Result<String, LoggerError> {
    let current_executable = std::env::current_exe()?;
    let filename = current_executable
        .file_name()
        .and_then(|filename_os_str| filename_os_str.to_str())
        .map(|filename_str| filename_str.to_string());
    filename.ok_or(LoggerError::LoggerRuntimeError {
        message: "Could not get current executable file name".to_string(),
    })
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

    reload_handle: tracing_subscriber::reload::Handle<EnvFilter, Registry>,
}

impl LogWorkerGuard {
    pub fn new(
        file_guard: Option<WorkerGuard>,
        stdout_guard: Option<WorkerGuard>,
        reload_handle: tracing_subscriber::reload::Handle<EnvFilter, Registry>,
    ) -> Self {
        Self { file_guard, stdout_guard, reload_handle }
    }

    pub fn reload(&self, env_filter_str: &str) -> Result<(), LoggerError> {
        let env_filter = EnvFilter::from_str(env_filter_str).map_err(|err| {
            LoggerError::LoggerConfigurationError {
                message: format!(
                    "Cannot parse the logger level: [{}]. err: {:?}",
                    env_filter_str, err
                ),
            }
        })?;
        self.reload_handle.reload(env_filter).map_err(|err| LoggerError::LoggerConfigurationError {
            message: format!("Cannot reload the logger configuration. err: {:?}", err),
        })
    }
}

/// Configures the underlying logger implementation and activates it.
pub fn setup_logger(
    logger_config: &LoggerConfig,
    config_dir: &str,
) -> Result<LogWorkerGuard, LoggerError> {
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

    let (stdout_subscriber, stdout_guard) = if logger_config.stdout_output {
        let (non_blocking, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
        (Some(Layer::new().with_ansi(false).with_writer(non_blocking)), Some(stdout_guard))
    } else {
        (None, None)
    };

    let apm_layer = if let Some(apm_server_url) = logger_config.apm_tracing.apm_server_url.clone() {
        let apm_server_api_credentials =
            logger_config.apm_tracing.read_api_credentials(config_dir)?;
        Some(tracing_elastic_apm::new_layer(
            get_current_service_name()?,
            tracing_elastic_apm::config::Config::new(apm_server_url).with_authorization(
                Authorization::ApiKey(ApiKey::new(
                    apm_server_api_credentials.id,
                    apm_server_api_credentials.key,
                )),
            ),
        ))
    } else {
        None
    };

    let subscriber = tracing_subscriber::registry()
        .with(reloadable_env_filter)
        .with(file_subscriber)
        .with(stdout_subscriber)
        .with(apm_layer);

    set_global_logger(subscriber)?;

    Ok(LogWorkerGuard { file_guard, stdout_guard, reload_handle: reloadable_env_filter_handle })
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
    fn split_the_file_path_should_file_if_directory_is_not_present() {
        assert!(path_to_dir_and_filename("filename").is_err());
    }

    #[test]
    fn should_get_correct_service_name() {
        assert!(get_current_service_name().unwrap().starts_with("tornado_common_logger"));
    }

    #[test]
    fn should_read_api_credentials_from_default_file() {
        let apm_tracing_config =
            ApmTracingConfig { apm_server_url: None, apm_server_credentials_filepath: None };
        let api_credentials = apm_tracing_config.read_api_credentials("./test_resources").unwrap();
        assert_eq!(
            api_credentials,
            ApiCredentials { id: "myid".to_string(), key: "mykey".to_string() }
        );
    }

    #[test]
    fn should_read_api_credentials_should_return_error_if_default_file_does_not_exist() {
        let apm_tracing_config =
            ApmTracingConfig { apm_server_url: None, apm_server_credentials_filepath: None };
        let res = apm_tracing_config.read_api_credentials("./");
        assert!(res.is_err());
    }

    #[test]
    fn should_read_api_credentials_should_return_error_if_file_is_not_correcly_formatted() {
        let apm_tracing_config = ApmTracingConfig {
            apm_server_url: None,
            apm_server_credentials_filepath: Some(
                "./test_resources/apm_server_api_credentials_wrong.json".to_string(),
            ),
        };
        let res = apm_tracing_config.read_api_credentials("./");
        assert!(res.is_err());
    }

    #[test]
    fn should_read_api_credentials_from_custom_file() {
        let apm_tracing_config = ApmTracingConfig {
            apm_server_url: None,
            apm_server_credentials_filepath: Some(
                "./test_resources/apm_server_api_credentials_custom.json".to_string(),
            ),
        };
        let api_credentials = apm_tracing_config.read_api_credentials("./").unwrap();
        assert_eq!(
            api_credentials,
            ApiCredentials { id: "custom_id".to_string(), key: "custom_key".to_string() }
        );
    }
}
