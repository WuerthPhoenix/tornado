use config_rs::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tornado_common::actors::nats_publisher::NatsPublisherConfig;
use tornado_common_logger::LoggerConfig;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SmsCollectorConfig {
    pub failed_sms_folder: PathBuf,
    pub tornado_connection_channel: Nats,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Nats {
    pub nats: NatsPublisherConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CollectorConfig {
    pub logger: LoggerConfig,
    pub sms_collector: SmsCollectorConfig,
}

pub fn build_config(config_dir: &str) -> Result<CollectorConfig, ConfigError> {
    let config_file_path = format!("{}/{}", &config_dir, "sms_collector.toml");
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}
