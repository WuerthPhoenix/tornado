use clap::{App, Arg, ArgMatches};
use config_rs::{Config, ConfigError, File};
use log::*;
use serde::{Deserialize, Serialize};
use std::fs;
use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;
use tornado_common::TornadoError;
use tornado_common_logger::LoggerConfig;
use tornado_common_api::{Payload, Value};
use tornado_common::actors::nats_publisher::NatsClientConfig;

pub const CONFIG_DIR_DEFAULT: Option<&'static str> =
    option_env!("TORNADO_NATS_JSON_COLLECTOR_CONFIG_DIR_DEFAULT");

pub fn arg_matches<'a>() -> ArgMatches<'a> {
    App::new("tornado_nats_json_collector")
        .arg(Arg::with_name("config-dir")
            .long("config-dir")
            .help("The filesystem folder where the Tornado Nats JSON collector configuration is saved")
            .default_value(CONFIG_DIR_DEFAULT.unwrap_or("/etc/tornado_nats_json_collector")))
        .arg(Arg::with_name("topics-dir")
            .long("topics-dir")
            .help("The folder where the topics Configurations are saved in JSON format; this folder is relative to the `config-dir`")
            .default_value("/topics/"))
        .get_matches()
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CollectorConfig {
    /// The logger configuration
    pub logger: LoggerConfig,
    pub nats_json_collector: NatsJsonCollectorConfig,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct NatsJsonCollectorConfig {
    pub message_queue_size: usize,

    pub nats_client: NatsClientConfig,

    pub tornado_connection_channel: TornadoConnectionChannel,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum TornadoConnectionChannel {
    Nats {
        nats_subject: String,
    },
    TCP {
        tcp_socket_ip: String,
        tcp_socket_port: u16,
    },
}

pub fn build_config(config_dir: &str) -> Result<CollectorConfig, ConfigError> {
    let config_file_path = format!("{}/{}", &config_dir, "nats_json_collector.toml");
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

pub fn read_topics_from_config(path: &str) -> Result<Vec<TopicConfig>, TornadoError> {
    info!("Loading topic configurations from path: [{}]", path);

    let paths = fs::read_dir(path).map_err(|e| TornadoError::ConfigurationError {
        message: format!("Cannot access config path [{}]: {}", path, e),
    })?;
    let mut topics = vec![];

    for path in paths {
        let filename = path
            .map_err(|e| TornadoError::ConfigurationError {
                message: format!("Cannot get the filename. Err: {}", e),
            })?
            .path();
        debug!("Loading topic configuration from file: [{}]", filename.display());
        let topic_body =
            fs::read_to_string(&filename).map_err(|e| TornadoError::ConfigurationError {
                message: format!("Unable to open the file [{}]. Err: {}", filename.display(), e),
            })?;
        trace!("Topic configuration body: \n{}", topic_body);
        topics.push(serde_json::from_str::<TopicConfig>(&topic_body).map_err(|e| {
            TornadoError::ConfigurationError {
                message: format!(
                    "Cannot build topic from json config: [{:?}] \n error: [{}]",
                    &topic_body, e
                ),
            }
        })?)
    }

    info!("Loaded {} topic(s) from [{}]", topics.len(), path);

    Ok(topics)
}

#[derive(Deserialize, Clone)]
pub struct TopicConfig {
    pub nats_topics: Vec<String>,
    pub collector_config: Option<EventConfig>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct EventConfig {
    pub event_type: Option<String>,
    pub payload: Option<Payload>,
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_read_configuration_from_file() {
        // Arrange
        let path = "./config/";

        // Act
        let config = build_config(path);

        // Assert
        assert!(config.is_ok())
    }

    #[test]
    fn should_read_all_topics_configurations_from_file() {
        // Arrange
        let path = "./config/topics";

        // Act
        let topics_config = read_topics_from_config(path).unwrap();

        // Assert
        assert_eq!(1 topics_config.len());
        assert_eq!(
            1,
            topics_config.iter().filter(|val| vec!["vsphere".to_owned(), "another_topic".to_owned()].eq(&val.nats_topics)).count()
        );
    }
}
