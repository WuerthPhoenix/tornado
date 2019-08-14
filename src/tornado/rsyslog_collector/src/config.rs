use config_rs::{Config, ConfigError, File};
use serde_derive::{Deserialize, Serialize};
use structopt::StructOpt;
use tornado_common_logger::LoggerConfig;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Conf {
    /// The filesystem folder where the Tornado Rsyslog Collector configuration is saved
    #[structopt(long, default_value = "/etc/tornado_rsyslog_collector")]
    pub config_dir: String,
}

impl Conf {
    pub fn build() -> Self {
        Conf::from_args()
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CollectorConfig {
    /// The logger configuration
    pub logger: LoggerConfig,
    pub rsyslog_collector: RsyslogCollectorConfig,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct RsyslogCollectorConfig {
    pub message_queue_size: usize,
    pub tornado_event_socket_ip: String,
    pub tornado_event_socket_port: u16,
}

pub fn build_config(config_dir: &str) -> Result<CollectorConfig, ConfigError> {
    let collector_config_path = format!("{}/{}", config_dir, "rsyslog_collector.toml");
    let mut s = Config::new();
    s.merge(File::with_name(&collector_config_path))?;
    s.try_into()
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

}
