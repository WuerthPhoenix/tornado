use crate::executor::icinga2::Icinga2ClientConfig;
use clap::{App, Arg, ArgMatches, SubCommand};
use config_rs::{Config, ConfigError, File};
use serde_derive::{Deserialize, Serialize};
use tornado_common_logger::LoggerConfig;
use tornado_engine_matcher::config::fs::FsMatcherConfigManager;
use tornado_engine_matcher::config::MatcherConfigManager;
use tornado_executor_archive::config::ArchiveConfig;
use std::sync::Arc;

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

fn build_archive_config(config_dir: &str) -> Result<ArchiveConfig, ConfigError> {
    let config_file_path = format!("{}/archive_executor.toml", config_dir);
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

fn build_icinga2_client_config(config_dir: &str) -> Result<Icinga2ClientConfig, ConfigError> {
    let config_file_path = format!("{}/icinga2_client_executor.toml", config_dir);
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

pub struct ComponentsConfig {
    pub matcher_config: Arc<dyn MatcherConfigManager>,
    pub tornado: GlobalConfig,
    pub archive_executor_config: ArchiveConfig,
    pub icinga2_executor_config: Icinga2ClientConfig,
}

pub fn parse_config_files(
    config_dir: &str,
    rules_dir: &str,
) -> Result<ComponentsConfig, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let matcher_config = Arc::new(build_matcher_config(config_dir, rules_dir));
    let tornado = build_config(config_dir)?;
    let archive_executor_config = build_archive_config(config_dir)?;
    let icinga2_executor_config = build_icinga2_client_config(config_dir)?;
    Ok(ComponentsConfig {
        matcher_config,
        tornado,
        archive_executor_config,
        icinga2_executor_config,
    })
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
            MatcherConfig::Ruleset { name, rules } => {
                assert_eq!("root", name);
                assert_eq!(5, rules.len());
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
    fn should_read_configurations_from_file() {
        // Arrange
        let config_dir = "./config";
        let rules_dir = "/rules.d";

        // Act
        let config = parse_config_files(config_dir, rules_dir).unwrap();

        // Assert
        assert_eq!(
            "https://127.0.0.1:5665/v1/actions",
            config.icinga2_executor_config.server_api_url
        )
    }

    #[test]
    fn should_read_archiver_configurations_from_file() {
        // Arrange
        let config_dir = "./config";

        // Act
        let config = build_archive_config(config_dir).unwrap();

        // Assert
        assert_eq!("./target/tornado-log", config.base_path)
    }

    #[test]
    fn should_read_icinga2_client_configurations_from_file() {
        // Arrange
        let config_dir = "./config";

        // Act
        let config = build_icinga2_client_config(config_dir).unwrap();

        // Assert
        assert_eq!("https://127.0.0.1:5665/v1/actions", config.server_api_url)
    }
}
