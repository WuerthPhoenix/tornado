use clap::{App, Arg, ArgMatches};
use config_rs::{Config, ConfigError, File};
use human_units::Size;
use log::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::num::NonZeroU16;
use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common::TornadoError;
use tornado_common_logger::LoggerConfig;

pub const CONFIG_DIR_DEFAULT: Option<&'static str> =
    option_env!("TORNADO_WEBHOOK_COLLECTOR_CONFIG_DIR_DEFAULT");

pub fn arg_matches() -> ArgMatches {
    App::new("tornado_webhook_collector")
        .arg(Arg::new("config-dir")
            .long("config-dir")
            .help("The filesystem folder where the Tornado Webhook collector configuration is saved")
            .default_value(CONFIG_DIR_DEFAULT.unwrap_or("/etc/tornado_webhook_collector")))
        .arg(Arg::new("webhooks-dir")
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

    pub tornado_connection_channel: Option<TornadoConnectionChannel>,

    pub tornado_event_socket_ip: Option<String>,
    pub tornado_event_socket_port: Option<u16>,

    pub server_bind_address: String,
    pub server_port: u32,

    pub workers: Option<NonZeroU16>,
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
                message: format!("Cannot get the filename. Err: {:?}", e),
            })?
            .path();
        debug!("Loading webhook configuration from file: [{}]", filename.display());
        let webhook_body =
            fs::read_to_string(&filename).map_err(|e| TornadoError::ConfigurationError {
                message: format!("Unable to open the file [{}]. Err: {:?}", filename.display(), e),
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

pub(crate) fn default_webhook_config_max_payload_size() -> Size {
    Size(5242880)
}

#[derive(Deserialize, Clone, Debug)]
pub struct WebhookConfig {
    pub id: String,
    pub token: String,
    #[serde(default = "default_webhook_config_max_payload_size")]
    pub max_payload_size: Size, // Support human-readable sizes with these suffixes: b, k, m, g, t, p, e
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

    #[test]
    fn should_have_valid_webhook_collector_config_workers() {
        // Arrange
        let config_workers_unspecified = r#"{"message_queue_size":1000,"tornado_connection_channel":null,"tornado_event_socket_ip":"127.0.0.1","tornado_event_socket_port":8081,"server_bind_address":"0.0.0.0","server_port":8080}"#;
        let config_workers_valid = r#"{"message_queue_size":1000,"tornado_connection_channel":null,"tornado_event_socket_ip":"127.0.0.1","tornado_event_socket_port":8081,"server_bind_address":"0.0.0.0","server_port":8080,"workers":4}"#;
        let config_workers_zero = r#"{"message_queue_size":1000,"tornado_connection_channel":null,"tornado_event_socket_ip":"127.0.0.1","tornado_event_socket_port":8081,"server_bind_address":"0.0.0.0","server_port":8080,"workers":0}"#;
        let config_workers_invalid = r#"{"message_queue_size":1000,"tornado_connection_channel":null,"tornado_event_socket_ip":"127.0.0.1","tornado_event_socket_port":8081,"server_bind_address":"0.0.0.0","server_port":8080,"workers":"invalid"}"#;

        // Act
        let res_workers_unspecified = serde_json::from_str::<WebhookCollectorConfig>(config_workers_unspecified);
        let res_workers_valid = serde_json::from_str::<WebhookCollectorConfig>(config_workers_valid);
        let res_workers_zero = serde_json::from_str::<WebhookCollectorConfig>(config_workers_zero);
        let res_workers_invalid = serde_json::from_str::<WebhookCollectorConfig>(config_workers_invalid);

        // Assert
        assert!(res_workers_unspecified.is_ok());
        assert!(res_workers_valid.is_ok());
        assert!(res_workers_zero.is_err());
        assert!(res_workers_invalid.is_err());
    }

    #[test]
    fn should_have_valid_webhook_config_max_payload_size() {
        // Arrange
        let config_null = r#"{"id":"hook_1","token":"hook_1_token","collector_config":{"event_type":"${map.first}","payload":{}}}"#;
        let config_numeric = r#"{"id":"hook_1","token":"hook_1_token","max_payload_size":"12345","collector_config":{"event_type":"${map.first}","payload":{}}}"#;
        let config_human_units = r#"{"id":"hook_1","token":"hook_1_token","max_payload_size":"2m","collector_config":{"event_type":"${map.first}","payload":{}}}"#;
        let config_invalid = r#"{"id":"hook_1","token":"hook_1_token","max_payload_size":"invalid","collector_config":{"event_type":"${map.first}","payload":{}}}"#;

        // Act
        let res_null = serde_json::from_str::<WebhookConfig>(config_null);
        let res_numeric = serde_json::from_str::<WebhookConfig>(config_numeric);
        let res_human_units = serde_json::from_str::<WebhookConfig>(config_human_units);
        let res_invalid = serde_json::from_str::<WebhookConfig>(config_invalid);

        // Assert
        assert!(res_null.is_ok());
        assert!(res_numeric.is_ok());
        assert!(res_human_units.is_ok());
        assert!(res_invalid.is_err());
    }
}
