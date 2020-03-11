use clap::{App, Arg, ArgMatches};
use config_rs::{Config, ConfigError, File};
use log::*;
use serde_derive::{Deserialize, Serialize};
use std::fs;
use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;
use tornado_common::actors::nats_streaming_publisher::StanPublisherConfig;
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common::TornadoError;
use tornado_common_logger::LoggerConfig;

pub const CONFIG_DIR_DEFAULT: Option<&'static str> =
    option_env!("TORNADO_WEBHOOK_COLLECTOR_CONFIG_DIR_DEFAULT");

pub fn arg_matches<'a>() -> ArgMatches<'a> {
    App::new("tornado_webhook_collector")
        .arg(Arg::with_name("config-dir")
            .long("config-dir")
            .help("The filesystem folder where the Tornado Webhook collector configuration is saved")
            .default_value(CONFIG_DIR_DEFAULT.unwrap_or("/etc/tornado_webhook_collector")))
        .arg(Arg::with_name("webhooks-dir")
            .long("webhooks-dir")
            .help("The folder where the Webhooks Configurations are saved in JSON format; this folder is relative to the `config-dir`")
            .default_value("/webhooks/"))
        .get_matches()
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CollectorConfig {
    /// The logger configuration
    pub logger: LoggerConfig,
    pub webhook_collector: WebhookCollectorConfig,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WebhookCollectorConfig {
    pub message_queue_size: usize,

    pub tornado_connection_channel: TornadoConnectionChannel,

    pub nats: Option<StanPublisherConfig>,

    pub tornado_event_socket_ip: Option<String>,
    pub tornado_event_socket_port: Option<u16>,

    pub server_bind_address: String,
    pub server_port: u32,
}

pub fn build_config(config_dir: &str) -> Result<CollectorConfig, ConfigError> {
    let config_file_path = format!("{}/{}", &config_dir, "webhook_collector.toml");
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

pub fn read_webhooks_from_config(path: &str) -> Result<Vec<WebhookConfig>, TornadoError> {
    info!("Loading webhook configurations from path: [{}]", path);

    let paths = fs::read_dir(path).map_err(|e| TornadoError::ConfigurationError {
        message: format!("Cannot access config path [{}]: {}", path, e),
    })?;
    let mut webhooks = vec![];

    for path in paths {
        let filename = path
            .map_err(|e| TornadoError::ConfigurationError {
                message: format!("Cannot get the filename. Err: {}", e),
            })?
            .path();
        debug!("Loading webhook configuration from file: [{}]", filename.display());
        let webhook_body =
            fs::read_to_string(&filename).map_err(|e| TornadoError::ConfigurationError {
                message: format!("Unable to open the file [{}]. Err: {}", filename.display(), e),
            })?;
        trace!("Webhook configuration body: \n{}", webhook_body);
        webhooks.push(serde_json::from_str::<WebhookConfig>(&webhook_body).map_err(|e| {
            TornadoError::ConfigurationError {
                message: format!(
                    "Cannot build webhook from json config: [{:?}] \n error: [{}]",
                    &webhook_body, e
                ),
            }
        })?)
    }

    info!("Loaded {} webhook(s) from [{}]", webhooks.len(), path);

    Ok(webhooks)
}

#[derive(Deserialize, Clone)]
pub struct WebhookConfig {
    pub id: String,
    pub token: String,
    pub collector_config: JMESPathEventCollectorConfig,
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
    fn should_read_all_webhooks_configurations_from_file() {
        // Arrange
        let path = "./config/webhooks";

        // Act
        let webhooks_config = read_webhooks_from_config(path).unwrap();

        // Assert
        assert_eq!(2, webhooks_config.len());
        assert_eq!(
            1,
            webhooks_config.iter().filter(|val| "bitbucket_test_repository".eq(&val.id)).count()
        );
        assert_eq!(
            1,
            webhooks_config.iter().filter(|val| "github_test_repository".eq(&val.id)).count()
        );
    }
}
