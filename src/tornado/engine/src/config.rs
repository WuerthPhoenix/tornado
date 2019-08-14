use crate::executor::icinga2::Icinga2ClientConfig;
use config_rs::{Config, ConfigError, File};
use serde_derive::{Deserialize, Serialize};
use structopt::StructOpt;
use tornado_common_logger::LoggerConfig;
use tornado_engine_matcher::config::fs::FsMatcherConfigManager;
use tornado_engine_matcher::config::MatcherConfigManager;
use tornado_executor_archive::config::ArchiveConfig;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Conf {
    /// The filesystem folder where the Tornado configuration is saved
    #[structopt(long, default_value = "/etc/tornado")]
    pub config_dir: String,

    /// The folder where the processing tree configuration is saved in JSON format;
    ///   this folder is relative to the `config_dir`.
    #[structopt(long, default_value = "/rules.d/")]
    pub rules_dir: String,

    #[structopt(subcommand)]
    pub command: Command,
}

#[derive(StructOpt, Debug, Clone)]
pub enum Command {
    #[structopt(name = "check")]
    /// Checks that the configuration is valid.
    Check,
    #[structopt(name = "daemon")]
    /// Starts the Tornado daemon
    Daemon {
        #[structopt(flatten)]
        daemon_config: DaemonCommandConfig,
    },
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(rename_all = "kebab-case")]
pub struct DaemonCommandConfig {
    /// The IP address where we will listen for incoming events.
    #[structopt(long, default_value = "127.0.0.1")]
    pub event_socket_ip: String,

    /// The port where we will listen for incoming events.
    #[structopt(long, default_value = "4747")]
    pub event_socket_port: u16,

    /// The IP address where the Tornado Web Server will listen for HTTP requests.
    ///  This is used, for example, by the monitoring endpoints.
    #[structopt(long, default_value = "127.0.0.1")]
    pub web_server_ip: String,

    /// The port where the Tornado Web Server will listen for HTTP requests.
    #[structopt(long, default_value = "4748")]
    pub web_server_port: u16,
}

impl Conf {
    pub fn build() -> Self {
        Conf::from_args()
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TornadoConfig {
    /// The logger configuration
    pub logger: LoggerConfig,
    pub archive_executor: ArchiveConfig,
    pub icinga2_executor: Icinga2ClientConfig
}

pub fn build_config(conf: &Conf) -> Result<TornadoConfig, ConfigError> {
    let config_file_path = format!("{}/tornado.toml", conf.config_dir);
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

pub struct ComponentsConfig {
    pub matcher_config: Box<MatcherConfigManager>,
    pub tornado: TornadoConfig,
}

pub fn parse_config_files(conf: &Conf) -> Result<ComponentsConfig, Box<std::error::Error>> {
    let matcher_config = Box::new(build_matcher_config(conf));
    let tornado = build_config(conf)?;
    Ok(ComponentsConfig { matcher_config, tornado })
}

fn build_matcher_config(conf: &Conf) -> impl MatcherConfigManager {
    FsMatcherConfigManager::new(format!("{}/{}", conf.config_dir, conf.rules_dir))
}

#[cfg(test)]
mod test {

    use super::*;
    use tornado_engine_matcher::config::fs::FsMatcherConfigManager;
    use tornado_engine_matcher::config::MatcherConfig;

    #[test]
    fn should_read_all_rule_configurations_from_file() {
        // Arrange
        let path = "./config/rules.d";

        // Act
        let config = FsMatcherConfigManager::new(path).read().unwrap();

        // Assert
        match config {
            MatcherConfig::Rules { rules } => {
                assert_eq!(4, rules.len());
                assert_eq!(1, rules.iter().filter(|val| "all_emails".eq(&val.name)).count());
                assert_eq!(
                    1,
                    rules.iter().filter(|val| "emails_with_temperature".eq(&val.name)).count()
                );
                assert_eq!(1, rules.iter().filter(|val| "archive_all".eq(&val.name)).count());
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_read_icinga2_client_configurations_from_file() {
        // Arrange
        let conf = Conf {
            config_dir: "./config".to_owned(),
            rules_dir: "/rules.d".to_owned(),
            command: Command::Check,
        };

        // Act
        let config = build_config(&conf).unwrap();

        // Assert
        assert_eq!("https://127.0.0.1:5665/v1/actions", config.icinga2_executor.server_api_url)
    }
}
