use config_rs::{Config, ConfigError, File};
use log::{info, trace};
use serde_derive::{Deserialize, Serialize};
use std::fs;
use structopt::StructOpt;
use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;
use tornado_common::TornadoError;
use tornado_common_logger::LoggerConfig;

#[derive(Debug, StructOpt, Clone)]
#[structopt(rename_all = "kebab-case")]
pub struct Io {
    /// The filesystem folder where the Tornado Icinga2 Collector configuration is saved
    #[structopt(long, default_value = "/etc/tornado_icinga2_collector")]
    pub config_dir: String,

    /// The folder where the Stream Configurations are saved in JSON format;
    ///   this folder is relative to the `config_dir`.
    #[structopt(long, default_value = "/streams")]
    pub streams_dir: String,

    /// Set the size of the in-memory queue where messages will be stored before being written
    /// to the output socket.
    #[structopt(long, default_value = "10000")]
    pub uds_mailbox_capacity: usize,

    /// The Tornado TCP address where outgoing events will be written
    #[structopt(long, default_value = "127.0.0.1:4747")]
    pub tornado_tcp_address: String,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Conf {
    #[structopt(flatten)]
    pub logger: LoggerConfig,

    #[structopt(flatten)]
    pub io: Io,
}

impl Conf {
    pub fn build() -> Self {
        Conf::from_args()
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Icinga2ClientConfig {
    /// The complete URL of the Icinga2 Event Stream API
    pub server_api_url: String,

    /// Username used to connect to the Icinga2 APIs
    pub username: String,

    /// Password used to connect to the Icinga2 APIs
    pub password: String,

    /// If true, the client will not verify the SSL certificate
    pub disable_ssl_verification: bool,

    /// In case of connection failure, how many milliseconds
    /// to wait before a new connection attempt.
    pub sleep_ms_between_connection_attempts: u64,
}

pub fn build_icinga2_client_config(
    config_file_path: &str,
) -> Result<Icinga2ClientConfig, ConfigError> {
    let mut s = Config::new();
    s.merge(File::with_name(config_file_path))?;
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
        let path = "./config/icinga2_collector.toml";

        // Act
        let config = build_icinga2_client_config(path).unwrap();

        // Assert
        assert_eq!("https://127.0.0.1:5665/v1/events", config.server_api_url)
    }
}
