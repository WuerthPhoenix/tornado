use crate::executor::icinga2::Icinga2ClientConfig;
use config_rs::{Config, ConfigError, File};
use log::{info, trace};
use std::fs;
use structopt::StructOpt;
use tornado_common::TornadoError;
use tornado_common_logger::LoggerConfig;
use tornado_engine_matcher::config::Rule;
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

pub fn read_rules_from_config(path: &str) -> Result<Vec<Rule>, TornadoError> {
    let paths = fs::read_dir(path).map_err(|e| TornadoError::ConfigurationError {
        message: format!("Cannot access config path [{}]: {}", path, e),
    })?;

    let mut rules = vec![];

    for path in paths {
        let filename = path
            .map_err(|e| TornadoError::ConfigurationError {
                message: format!("Cannot get the filename. Err: {}", e),
            })?
            .path();

        info!("Loading rule from file: [{}]", filename.display());
        let rule_body =
            fs::read_to_string(&filename).map_err(|e| TornadoError::ConfigurationError {
                message: format!("Unable to open the file [{}]. Err: {}", filename.display(), e),
            })?;

        trace!("Rule body: \n{}", rule_body);
        rules.push(Rule::from_json(&rule_body).map_err(|e| TornadoError::ConfigurationError {
            message: format!(
                "Cannot build webhook from json config: [{:?}] \n error: [{}]",
                &rule_body, e
            ),
        })?)
    }

    info!("Loaded {} rule(s) from [{}]", rules.len(), path);

    Ok(rules)
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_read_all_rule_configurations_from_file() {
        // Arrange
        let path = "./config/rules.d";

        // Act
        let rules_config = read_rules_from_config(path).unwrap();

        // Assert
        assert_eq!(4, rules_config.len());
        assert_eq!(1, rules_config.iter().filter(|val| "all_emails".eq(&val.name)).count());
        assert_eq!(
            1,
            rules_config.iter().filter(|val| "emails_with_temperature".eq(&val.name)).count()
        );
        assert_eq!(1, rules_config.iter().filter(|val| "archive_all".eq(&val.name)).count());
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
