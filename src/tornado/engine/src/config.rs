use crate::executor::icinga2::Icinga2ClientConfig;
use config_rs::{Config, ConfigError, File};
use structopt::StructOpt;
use tornado_common_logger::LoggerConfig;
use tornado_executor_archive::config::ArchiveConfig;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Io {
    /// The filesystem folder where the Tornado configuration is saved
    #[structopt(long, default_value = "/etc/tornado")]
    pub config_dir: String,

    /// The folder where the Rules are saved in JSON format;
    ///   this folder is relative to the `config_dir`.
    #[structopt(long, default_value = "/rules.d/")]
    pub rules_dir: String,

    /// The Unix Socket path where we will listen for incoming events.
    #[structopt(long, default_value = "/var/run/tornado/tornado.sock")]
    pub uds_path: String,

    /// The Unix Socket path where we will listen for incoming snmptrapd events.
    #[structopt(long, default_value = "/var/run/tornado/tornado_snmptrapd.sock")]
    pub snmptrapd_uds_path: String,
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

pub fn build_archive_config(config_file_path: &str) -> Result<ArchiveConfig, ConfigError> {
    let mut s = Config::new();
    s.merge(File::with_name(config_file_path))?;
    s.try_into()
}

pub fn build_icinga2_client_config(
    config_file_path: &str,
) -> Result<Icinga2ClientConfig, ConfigError> {
    let mut s = Config::new();
    s.merge(File::with_name(config_file_path))?;
    s.try_into()
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
