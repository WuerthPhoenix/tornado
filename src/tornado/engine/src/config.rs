use crate::executor::icinga2::Icinga2ClientConfig;
use config_rs::{Config, ConfigError, File};
use failure::Fail;
use structopt::StructOpt;
use tornado_common_logger::LoggerConfig;
use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::error::MatcherError;
use tornado_executor_archive::config::ArchiveConfig;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Conf {
    #[structopt(flatten)]
    pub logger: LoggerConfig,

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

    /// The IP address of the Tornado web server.
    #[structopt(long, default_value = "127.0.0.1")]
    pub web_server_ip: String,

    /// The port of the Tornado web server.
    #[structopt(long, default_value = "4748")]
    pub web_server_port: u16,
}

impl Conf {
    pub fn build() -> Self {
        Conf::from_args()
    }
}

pub struct ComponentsConfig {
    pub matcher: MatcherConfig,
    pub archive: ArchiveConfig,
    pub icinga2_client: Icinga2ClientConfig,
}

pub fn parse_config_files(conf: &Conf) -> Result<ComponentsConfig, Box<std::error::Error>> {
    let matcher = build_matcher_config(&conf).map_err(|e| e.compat())?;
    let archive = build_archive_config(&conf)?;
    let icinga2_client = build_icinga2_client_config(&conf)?;
    Ok(ComponentsConfig { matcher, archive, icinga2_client })
}

fn build_archive_config(conf: &Conf) -> Result<ArchiveConfig, ConfigError> {
    let config_file_path = format!("{}/archive_executor.toml", conf.config_dir);
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

fn build_icinga2_client_config(conf: &Conf) -> Result<Icinga2ClientConfig, ConfigError> {
    let config_file_path = format!("{}/icinga2_client_executor.toml", conf.config_dir);
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

fn build_matcher_config(conf: &Conf) -> Result<MatcherConfig, MatcherError> {
    MatcherConfig::read_from_dir(&format!("{}/{}", conf.config_dir, conf.rules_dir))
}

#[cfg(test)]
mod test {

    use super::*;
    use tornado_engine_matcher::config::MatcherConfig;

    #[test]
    fn should_read_all_rule_configurations_from_file() {
        // Arrange
        let path = "./config/rules.d";

        // Act
        let config = MatcherConfig::read_from_dir(path).unwrap();

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
            logger: Default::default(),
            config_dir: "./config".to_owned(),
            rules_dir: "/rules.d".to_owned(),
            command: Command::Check,
        };

        // Act
        let config = build_icinga2_client_config(&conf).unwrap();

        // Assert
        assert_eq!("https://127.0.0.1:5665/v1/actions", config.server_api_url)
    }
}
