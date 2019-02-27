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
    #[structopt(long, default_value = "/etc/tornado_icinga2_collector/")]
    pub config_dir: String,

    /// The folder where the Stream Configurations are saved in JSON format;
    ///   this folder is relative to the `config_dir`.
    #[structopt(long, default_value = "/streams/")]
    pub streams_dir: String,

    /// Set the size of the in-memory queue where messages will be stored before being written
    /// to the output socket.
    #[structopt(long, default_value = "10000")]
    pub uds_mailbox_capacity: usize,

    /// The Unix Socket path where outgoing events will be written
    #[structopt(long, default_value = "/var/run/tornado/tornado.sock")]
    pub uds_path: String,

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
    types: Vec<EventType>,
    queue: String,
    filter: Option<String>,
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
    fn should_read_stream_configurations_from_config() {
        // Arrange
        let path = "./config/streams";

        // Act
        let streams_config = read_streams_from_config(path).unwrap();

        // Assert
        assert_eq!(1, streams_config.len());
    }
}
