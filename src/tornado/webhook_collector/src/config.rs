use log::{info, trace};
use serde_derive::Deserialize;
use std::fs;
use std::io;
use structopt::StructOpt;
use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;
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

    /// The Unix Socket path where we will write the Tornado events.
    #[structopt(long, default_value = "/var/run/tornado/tornado.sock")]
    pub uds_path: String,

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

pub fn read_webhooks_from_config(path: &str) -> io::Result<Vec<WebhookConfig>> {
    info!("Loading webhook configurations from path: [{}]", path);

    let paths = fs::read_dir(path)?;
    let mut webhooks = vec![];

    for path in paths {
        let filename = path?.path();
        info!("Loading webhook configuration from file: [{}]", filename.display());
        let webhook_body = fs::read_to_string(&filename)?;
        trace!("Webhook configuration body: \n{}", webhook_body);
        webhooks.push(serde_json::from_str::<WebhookConfig>(&webhook_body)?);
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
        assert_eq!("bitbucket_test_repository", webhooks_config[0].id);
        assert_eq!("github_test_repository", webhooks_config[1].id);
    }
}
