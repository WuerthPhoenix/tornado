use log::{info, trace};
use serde_derive::Deserialize;
use std::fs;
use structopt::StructOpt;
use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;
use tornado_common::TornadoError;
use tornado_common_logger::LoggerConfig;

#[derive(Debug, StructOpt, Clone)]
#[structopt(rename_all = "kebab-case")]
pub struct Io {
    /// The filesystem folder where the Tornado Webhook collector configuration is saved
    #[structopt(long, default_value = "/etc/tornado_webhook_collector/")]
    pub config_dir: String,

    /// The folder where the Webhooks Configurations are saved in JSON format;
    ///   this folder is relative to the `config_dir`.
    #[structopt(long, default_value = "/webhooks/")]
    pub webhooks_dir: String,

    /// Set the size of the in-memory queue where messages will be stored before being written
    /// to the output socket.
    #[structopt(long, default_value = "10000")]
    pub message_queue_size: usize,

    /// The Tornado TCP address where outgoing events will be written
    #[structopt(long, default_value = "127.0.0.1:4747")]
    pub tornado_tcp_address: String,

    /// IP to bind the HTTP server to.
    #[structopt(long, default_value = "0.0.0.0")]
    pub bind_address: String,

    /// The port to be use by the HTTP Server.
    #[structopt(long, default_value = "8080")]
    pub server_port: u32,
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
        info!("Loading webhook configuration from file: [{}]", filename.display());
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
