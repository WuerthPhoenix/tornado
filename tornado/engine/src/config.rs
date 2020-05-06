use crate::executor::icinga2::Icinga2ClientConfig;
use clap::{App, Arg, ArgMatches, SubCommand};
use config_rs::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tornado_common::actors::nats_subscriber::NatsSubscriberConfig;
use tornado_common_logger::LoggerConfig;
use tornado_engine_api::auth::Permission;
use tornado_engine_matcher::config::fs::FsMatcherConfigManager;
use tornado_executor_archive::config::ArchiveConfig;
use tornado_executor_elasticsearch::config::ElasticsearchConfig;

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
        .arg(Arg::with_name("drafts-dir")
            .long("drafts-dir")
            .help("The folder where the configuration drafts are saved in JSON format. This folder is relative to the `config-dir`")
            .default_value("/drafts/"))
        .subcommand(SubCommand::with_name("daemon" )
            .help("Starts the Tornado daemon"))
        .subcommand(SubCommand::with_name("check" )
            .help("Checks that the configuration is valid"))
        .get_matches()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ThreadPoolConfig {
    CPU { factor: f64 },
    Fixed { size: isize },
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        ThreadPoolConfig::CPU { factor: 1.0 }
    }
}

impl ThreadPoolConfig {
    pub fn get_threads_count(&self) -> usize {
        let count = match self {
            ThreadPoolConfig::CPU { factor } => {
                ((num_cpus::get() as f64) * *factor).ceil() as isize
            }
            ThreadPoolConfig::Fixed { size } => *size,
        };
        if count > 0 {
            count as usize
        } else {
            1
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DaemonCommandConfig {
    pub event_tcp_socket_enabled: Option<bool>,
    pub event_socket_ip: Option<String>,
    pub event_socket_port: Option<u16>,

    pub nats_enabled: Option<bool>,
    pub nats: Option<NatsSubscriberConfig>,

    pub web_server_ip: String,
    pub web_server_port: u16,

    pub message_queue_size: usize,

    pub thread_pool_config: Option<ThreadPoolConfig>,

    pub auth: AuthConfig,
}

impl DaemonCommandConfig {
    pub fn is_event_tcp_socket_enabled(&self) -> bool {
        self.event_tcp_socket_enabled.unwrap_or(true)
    }

    pub fn is_nats_enabled(&self) -> bool {
        self.nats_enabled.unwrap_or(false)
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct AuthConfig {
    pub role_permissions: BTreeMap<String, Vec<Permission>>,
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

fn build_elasticsearch_config(config_dir: &str) -> Result<ElasticsearchConfig, ConfigError> {
    let config_file_path = format!("{}/elasticsearch_executor.toml", config_dir);
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

pub struct ComponentsConfig {
    pub matcher_config: Arc<FsMatcherConfigManager>,
    pub tornado: GlobalConfig,
    pub archive_executor_config: ArchiveConfig,
    pub icinga2_executor_config: Icinga2ClientConfig,
    pub elasticsearch_executor_config: ElasticsearchConfig,
}

pub fn parse_config_files(
    config_dir: &str,
    rules_dir: &str,
    drafts_dir: &str,
) -> Result<ComponentsConfig, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let matcher_config = Arc::new(build_matcher_config(config_dir, rules_dir, drafts_dir));
    let tornado = build_config(config_dir)?;
    let archive_executor_config = build_archive_config(config_dir)?;
    let icinga2_executor_config = build_icinga2_client_config(config_dir)?;
    let elasticsearch_executor_config = build_elasticsearch_config(config_dir)?;
    Ok(ComponentsConfig {
        matcher_config,
        tornado,
        archive_executor_config,
        icinga2_executor_config,
        elasticsearch_executor_config,
    })
}

fn build_matcher_config(
    config_dir: &str,
    rules_dir: &str,
    drafts_dir: &str,
) -> FsMatcherConfigManager {
    FsMatcherConfigManager::new(
        format!("{}/{}", config_dir, rules_dir),
        format!("{}/{}", config_dir, drafts_dir),
    )
}

#[cfg(test)]
mod test {

    use super::*;
    use tornado_engine_matcher::config::fs::FsMatcherConfigManager;
    use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigReader};

    #[test]
    fn should_read_configuration_from_file() {
        // Arrange
        let path = "./config/";

        // Act
        let config = build_config(path).unwrap();

        // Assert
        assert_eq!(
            vec![Permission::ConfigEdit, Permission::ConfigView],
            config.tornado.daemon.auth.role_permissions["ADMIN"]
        )
    }

    #[test]
    fn should_read_all_rule_configurations_from_file() {
        // Arrange
        let path = "./config/rules.d";
        let drafts_path = "./config/drafts";

        // Act
        let config = FsMatcherConfigManager::new(path, drafts_path).get_config().unwrap();

        // Assert
        match config {
            MatcherConfig::Ruleset { name, rules } => {
                assert_eq!("root", name);
                assert_eq!(6, rules.len());
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
        let drafts_dir = "/drafts";

        // Act
        let config = parse_config_files(config_dir, rules_dir, drafts_dir).unwrap();

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

    #[test]
    fn channel_config_getters_should_correctly_extract_value() {
        // Arrange
        let daemon_configs = DaemonCommandConfig {
            event_tcp_socket_enabled: Some(false),
            event_socket_ip: None,
            event_socket_port: None,
            nats_enabled: Some(true),
            nats: None,
            web_server_ip: "".to_string(),
            web_server_port: 0,
            message_queue_size: 0,
            thread_pool_config: None,
            auth: AuthConfig::default(),
        };

        // Act
        let event_tcp_socket_enabled = daemon_configs.is_event_tcp_socket_enabled();
        let nats_enabled = daemon_configs.is_nats_enabled();

        // Assert
        assert_eq!(event_tcp_socket_enabled, false);
        assert_eq!(nats_enabled, true);
    }

    #[test]
    fn channel_config_getters_should_correctly_handle_none() {
        // Arrange
        let daemon_configs = DaemonCommandConfig {
            event_tcp_socket_enabled: None,
            event_socket_ip: None,
            event_socket_port: None,
            nats_enabled: None,
            nats: None,
            web_server_ip: "".to_string(),
            web_server_port: 0,
            message_queue_size: 0,
            thread_pool_config: None,
            auth: AuthConfig::default(),
        };

        // Act
        let event_tcp_socket_enabled = daemon_configs.is_event_tcp_socket_enabled();
        let nats_enabled = daemon_configs.is_nats_enabled();

        // Assert
        assert_eq!(event_tcp_socket_enabled, true);
        assert_eq!(nats_enabled, false);
    }

    #[test]
    fn thread_pool_config_should_never_return_less_than_one() {
        assert_eq!(1, ThreadPoolConfig::Fixed { size: -3 }.get_threads_count());
        assert_eq!(1, ThreadPoolConfig::Fixed { size: 0 }.get_threads_count());
        assert_eq!(1, ThreadPoolConfig::CPU { factor: 0.0 }.get_threads_count());
        assert_eq!(1, ThreadPoolConfig::CPU { factor: -10.0 }.get_threads_count());
    }

    #[test]
    fn thread_pool_config_fixed_should_return_size() {
        let random: usize = rand::random();
        assert_eq!(random, ThreadPoolConfig::Fixed { size: (random as isize) }.get_threads_count());
    }

    #[test]
    fn thread_pool_config_cpu_should_return_count_by_factor() {
        let cpus = num_cpus::get();
        assert_eq!(cpus, ThreadPoolConfig::CPU { factor: 1.0 }.get_threads_count());
        assert_eq!((cpus / 2) as usize, ThreadPoolConfig::CPU { factor: 0.5 }.get_threads_count());
        assert_eq!((cpus * 3) as usize, ThreadPoolConfig::CPU { factor: 3.0 }.get_threads_count());
    }

    #[test]
    fn thread_pool_config_should_default_tocpu_count() {
        let cpus = num_cpus::get();
        assert_eq!(cpus, ThreadPoolConfig::default().get_threads_count());
    }
}
