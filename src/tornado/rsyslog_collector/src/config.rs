use config_rs::{Config, ConfigError, File};
use serde_derive::{Deserialize, Serialize};
use structopt::StructOpt;
use tornado_common_logger::LoggerConfig;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Io {
    /// The filesystem folder where the Tornado Rsyslog Collector configuration is saved
    #[structopt(long, default_value = "/etc/tornado_rsyslog_collector")]
    pub config_dir: String,

    /// Set the size of the in-memory queue where messages will be stored before being written
    /// to the output socket.
    #[structopt(long, default_value = "10000")]
    pub message_queue_size: usize,

    /// The Tornado IP address where outgoing events will be written
    #[structopt(long, default_value = "127.0.0.1")]
    pub tornado_event_socket_ip: String,

    /// The Tornado port where outgoing events will be written
    #[structopt(long, default_value = "4747")]
    pub tornado_event_socket_port: u16,
}

#[derive(Debug, StructOpt)]
pub struct Conf {
    #[structopt(flatten)]
    pub io: Io,
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
}

pub fn build_config(config_file_path: &str) -> Result<CollectorConfig, ConfigError> {
    let mut s = Config::new();
    s.merge(File::with_name(config_file_path))?;
    s.try_into()
}
