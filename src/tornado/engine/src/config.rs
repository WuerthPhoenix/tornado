use crate::executor::icinga2::Icinga2ClientConfig;
use clap::{App, Arg, ArgMatches, SubCommand};
use config_rs::{Config, ConfigError, File};
use serde_derive::{Deserialize, Serialize};
use tornado_common_logger::LoggerConfig;
use tornado_engine_matcher::config::fs::FsMatcherConfigManager;
use tornado_engine_matcher::config::MatcherConfigManager;
use tornado_executor_archive::config::ArchiveConfig;

pub const CONFIG_DIR_DEFAULT: Option<&'static str> = option_env!("TORNADO_CONFIG_DIR_DEFAULT");

pub fn arg_matches<'a>() -> ArgMatches<'a> {
    App::new("tornado_daemon")
        .arg(Arg::with_name("config-dir")
            .long("config-dir")
            .help("The filesystem folder where the Tornado configuration is saved")
            .default_value(CONFIG_DIR_DEFAULT.unwrap_or("/etc/tornado")))
        .arg(Arg::with_name("rules-dir")
            .long("rules-dir")
            .help("The folder where the processing tree configuration is saved in JSON format. This folder is relative to the `config-dir`")
            .default_value("/rules.d/"))
        .subcommand(SubCommand::with_name("daemon" )
            .help("Starts the Tornado daemon"))
        .subcommand(SubCommand::with_name("check" )
            .help("Checks that the configuration is valid"))
        .get_matches()
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DaemonCommandConfig {
    pub event_socket_ip: String,
    pub event_socket_port: u16,
    pub web_server_ip: String,
    pub web_server_port: u16,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct GlobalConfig {
    /// The logger configuration
    pub logger: LoggerConfig,
    pub tornado: TornadoConfig,
    pub archive_executor: ArchiveConfig,
    pub icinga2_executor: Icinga2ClientConfig,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TornadoConfig {
    pub daemon: DaemonCommandConfig,
}

pub fn build_config(config_dir: &str) -> Result<GlobalConfig, ConfigError> {
    let config_file_path = format!("{}/tornado.toml", config_dir);
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

pub struct ComponentsConfig {
    pub matcher_config: Box<MatcherConfigManager>,
    pub tornado: GlobalConfig,
}

pub fn parse_config_files(
    config_dir: &str,
    rules_dir: &str,
) -> Result<ComponentsConfig, Box<std::error::Error>> {
    let matcher_config = Box::new(build_matcher_config(config_dir, rules_dir));
    let tornado = build_config(config_dir)?;
    Ok(ComponentsConfig { matcher_config, tornado })
}

fn build_matcher_config(config_dir: &str, rules_dir: &str) -> impl MatcherConfigManager {
    FsMatcherConfigManager::new(format!("{}/{}", config_dir, rules_dir))
}

#[cfg(test)]
mod test {

    use super::*;
    use tornado_engine_matcher::config::fs::FsMatcherConfigManager;
    use tornado_engine_matcher::config::MatcherConfig;

    #[test]
    fn should_read_configuration_from_file() {
        // Arrange
        let path = "./config/";

        // Act
        let config = build_config(path);

        // Assert
        assert!(config.is_ok())
    }

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
        let config_dir = "./config";
        let rules_dir = "/rules.d";

        // Act
        let config = parse_config_files(config_dir, rules_dir).unwrap();

        // Assert
        assert_eq!(
            "https://127.0.0.1:5665/v1/actions",
            config.tornado.icinga2_executor.server_api_url
        )
    }
}
