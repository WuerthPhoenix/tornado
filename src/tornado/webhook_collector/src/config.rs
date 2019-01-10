use config_rs::{Config, ConfigError, File};
use structopt::StructOpt;
use serde_derive::{Deserialize};
use tornado_common_logger::LoggerConfig;
use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Io {
    /// The filesystem folder where the Tornado Webhook collector configuration is saved
    #[structopt(long, default_value = "/etc/tornado_webhook_collector/")]
    pub config_dir: String,

    /// The Unix Socket path where we will write the Tornado events.
    #[structopt(long, default_value = "/var/run/tornado/tornado.sock")]
    pub uds_path: String,
}

#[derive(Debug, StructOpt)]
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

pub fn build_config(config_file_path: &str) -> Result<WebhookConfig, ConfigError> {
    let mut s = Config::new();
    s.merge(File::with_name(config_file_path))?;
    s.try_into()
}

/*
{
    id: "a",
    token: "b",
    collector_config: {
        "event_type": "${commits[0].committer.name}",
        "payload": {
        "ref": "${ref}",
        "repository_name": "${repository.name}"
        }
    }
}
*/

#[derive(Deserialize)]
pub struct WebhookConfig {
    id: String,
    token: String,
    collector_config: JMESPathEventCollectorConfig,
}
