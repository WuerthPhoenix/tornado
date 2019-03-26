use self::command::Command;
use crate::executor::icinga2::Icinga2ClientConfig;
use config_rs::{Config, ConfigError, File};
use failure::Fail;
use structopt::StructOpt;
use tornado_common_logger::LoggerConfig;
use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::error::MatcherError;
use tornado_executor_archive::config::ArchiveConfig;

mod command;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Io {
    /// The filesystem folder where the Tornado configuration is saved
    #[structopt(long, default_value = "/etc/tornado")]
    pub config_dir: String,

    /// The folder where the processing tree configuration is saved in JSON format;
    ///   this folder is relative to the `config_dir`.
    #[structopt(long, default_value = "/rules.d/")]
    pub rules_dir: String,

    /// The IP address where we will listen for incoming events.
    #[structopt(long, default_value = "127.0.0.1")]
    pub event_socket_ip: String,

    /// The port where we will listen for incoming events.
    #[structopt(long, default_value = "4747")]
    pub event_socket_port: u16,

    /// The IP address where we will listen for incoming snmptrapd events.
    #[structopt(long, default_value = "127.0.0.1")]
    pub snmptrapd_socket_ip: String,

    /// The port where we will listen for incoming snmptrapd events.
    #[structopt(long, default_value = "4748")]
    pub snmptrapd_socket_port: u16,
}

#[derive(Debug, StructOpt)]
pub struct Conf {
    #[structopt(flatten)]
    pub logger: LoggerConfig,

    #[structopt(flatten)]
    pub io: Io,

    #[structopt(subcommand)]
    pub command: Command,
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

// Todo: use a struct
pub fn parse_config_files(conf: &Conf) -> Result<ComponentsConfig, Box<std::error::Error>> {
    let matcher = build_matcher_config(&conf).map_err(|e| e.compat())?;
    let archive = build_archive_config(&conf)?;
    let icinga2_client = build_icinga2_client_config(&conf)?;
    Ok(ComponentsConfig { matcher, archive, icinga2_client })
}

fn build_archive_config(conf: &Conf) -> Result<ArchiveConfig, ConfigError> {
    let config_file_path = format!("{}/archive_executor.toml", conf.io.config_dir);
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

fn build_icinga2_client_config(conf: &Conf) -> Result<Icinga2ClientConfig, ConfigError> {
    let config_file_path = format!("{}/icinga2_client_executor.toml", conf.io.config_dir);
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

fn build_matcher_config(conf: &Conf) -> Result<MatcherConfig, MatcherError> {
    MatcherConfig::read_from_dir(&format!("{}/{}", conf.io.config_dir, conf.io.rules_dir))
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
        let path = "./config/icinga2_client_executor.toml";

        // Act
        let config = build_icinga2_client_config(path).unwrap();

        // Assert
        assert_eq!("https://127.0.0.1:5665/v1/actions", config.server_api_url)
    }
}
