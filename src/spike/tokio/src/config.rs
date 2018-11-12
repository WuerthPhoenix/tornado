use config_rs::{Config, ConfigError, Environment, File};
use tornado_common_logger::LoggerConfig;

#[derive(Debug, Deserialize)]
pub struct Io {
    pub uds_socket_path: String,
    pub json_rules_path: String,
}

#[derive(Debug, Deserialize)]
pub struct Conf {
    pub logger: LoggerConfig,
    pub io: Io,
}

impl Conf {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(File::with_name("./config/config"))?;
        s.merge(Environment::with_prefix("engine"))?;
        s.try_into()
    }
}
