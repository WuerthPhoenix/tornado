use clap::{App, Arg, ArgMatches};
use config_rs::{Config, ConfigError, File};
use log::{info, trace};
use serde_derive::{Deserialize, Serialize};
use std::fs;
use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common::TornadoError;
use tornado_common_logger::LoggerConfig;

pub const CONFIG_DIR_DEFAULT: Option<&'static str> =
    option_env!("TORNADO_ICINGA2_COLLECTOR_CONFIG_DIR_DEFAULT");

pub fn arg_matches<'a>() -> ArgMatches<'a> {
    App::new("tornado_icinga2_collector")
        .arg(Arg::with_name("config-dir")
            .long("config-dir")
            .help("The filesystem folder where the Tornado Icinga2 Collector configuration is saved")
            .default_value(CONFIG_DIR_DEFAULT.unwrap_or("/etc/tornado_icinga2_collector")))
        .arg(Arg::with_name("streams-dir")
            .long("streams-dir")
            .help("The folder where the Stream Configurations are saved in JSON format; this folder is relative to the `config-dir`")
            .default_value("/streams"))
        .get_matches()
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Icinga2CollectorConfig {
    pub message_queue_size: usize,
    pub connection: Icinga2ClientConfig,

    pub tornado_connection_channel: Option<TornadoConnectionChannel>,

    pub tornado_event_socket_ip: Option<String>,
    pub tornado_event_socket_port: Option<u16>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Icinga2ClientConfig {
    pub server_api_url: String,
    pub username: String,
    pub password: String,
    pub disable_ssl_verification: bool,
    pub sleep_ms_between_connection_attempts: u64,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CollectorConfig {
    /// The logger configuration
    pub logger: LoggerConfig,

    /// The icinga2 client configuration
    pub icinga2_collector: Icinga2CollectorConfig,
}

pub fn build_config(config_dir: &str) -> Result<CollectorConfig, ConfigError> {
    let config_file_path = format!("{}/{}", config_dir, "icinga2_collector.toml");
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

pub fn read_streams_from_config(path: &str) -> Result<Vec<StreamConfig>, TornadoError> {
    info!("Loading Stream configurations from path: [{}]", path);

    let paths = fs::read_dir(path).map_err(|e| TornadoError::ConfigurationError {
        message: format!("Cannot access config path [{}]: {}", path, e),
    })?;
    let mut streams = vec![];

    for path in paths {
        let filename = path
            .map_err(|e| TornadoError::ConfigurationError {
                message: format!("Cannot get the filename. Err: {}", e),
            })?
            .path();

        if let Some(name) = filename.to_str() {
            if !name.ends_with(".json") {
                info!("Configuration file [{}] is ignored.", filename.display());
                continue;
            }
        } else {
            return Err(TornadoError::ConfigurationError {
                message: format!("Cannot process filename of [{}].", filename.display()),
            });
        }

        info!("Loading stream configuration from file: [{}]", filename.display());

        let stream_body =
            fs::read_to_string(&filename).map_err(|e| TornadoError::ConfigurationError {
                message: format!("Unable to open the file [{}]. Err: {}", filename.display(), e),
            })?;
        trace!("Stream configuration body: \n{}", stream_body);
        streams.push(serde_json::from_str::<StreamConfig>(&stream_body).map_err(|e| {
            TornadoError::ConfigurationError {
                message: format!(
                    "Cannot build stream from json config: [{:?}] \n error: [{}]",
                    &stream_body, e
                ),
            }
        })?)
    }

    info!("Loaded {} stream(s) from [{}]", streams.len(), path);

    Ok(streams)
}

#[derive(Deserialize, Clone)]
pub struct StreamConfig {
    pub stream: Stream,
    pub collector_config: JMESPathEventCollectorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stream {
    pub types: Vec<EventType>,
    pub queue: String,
    pub filter: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum EventType {
    CheckResult,
    StateChange,
    Notification,
    AcknowledgementSet,
    AcknowledgementCleared,
    CommentAdded,
    CommentRemoved,
    DowntimeAdded,
    DowntimeRemoved,
    DowntimeStarted,
    DowntimeTriggered,
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_read_stream_configurations_from_config_json_files() {
        // Arrange
        let path = "./config/streams";

        // Act
        let streams_config = read_streams_from_config(path).unwrap();

        // Assert
        assert_eq!(3, streams_config.len());
    }

    #[test]
    fn should_read_icinga2_config_from_file() {
        // Arrange
        let path = "./config/";

        // Act
        let config = build_config(path).unwrap();

        // Assert
        assert_eq!(
            "https://127.0.0.1:5665/v1/events",
            config.icinga2_collector.connection.server_api_url
        )
    }
}
