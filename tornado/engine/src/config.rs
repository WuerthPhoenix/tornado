use clap::Clap;
use config_rs::{Config, ConfigError, File};
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use tornado_common::{
    actors::nats_subscriber::NatsSubscriberConfig, command::retry::RetryStrategy,
};
use tornado_common_logger::LoggerConfig;
use tornado_engine_api::auth::Permission;
use tornado_engine_matcher::config::fs::FsMatcherConfigManager;
use tornado_executor_archive::config::ArchiveConfig;
use tornado_executor_director::config::DirectorClientConfig;
use tornado_executor_elasticsearch::config::ElasticsearchConfig;
use tornado_executor_icinga2::config::Icinga2ClientConfig;
use tornado_executor_smart_monitoring_check_result::config::SmartMonitoringCheckResultConfig;
use crate::enrich::nats::NatsExtractor;

pub const CONFIG_DIR_DEFAULT: Option<&'static str> = option_env!("TORNADO_CONFIG_DIR_DEFAULT");

#[derive(Clap, Debug)]
#[clap(name = "tornado")]
pub struct Opt {
    #[clap(long = "config-dir")]
    /// The filesystem folder where the Tornado configuration is saved
    config_dir: Option<String>,
    #[clap(long = "rules-dir")]
    /// The folder where the processing tree configuration is saved in JSON format. This folder is relative to the `config-dir`
    rules_dir: Option<String>,
    #[clap(long = "drafts-dir")]
    /// The folder where the configuration drafts are saved in JSON format. This folder is relative to the `config-dir`
    drafts_dir: Option<String>,
    #[clap(subcommand)]
    pub command: SubCommand,
}

impl Opt {
    pub fn config_dir(&self) -> &str {
        let config_dir = match &self.config_dir {
            Some(config_dir) => config_dir,
            None => CONFIG_DIR_DEFAULT.unwrap_or("/etc/tornado"),
        };
        // here the logger is not yet available, so print it to stdout
        println!("Start with configuration directory: [{}]", config_dir);
        config_dir
    }

    pub fn rules_dir(&self) -> &str {
        let rules_dir = match &self.rules_dir {
            Some(rules_dir) => rules_dir,
            None => "/rules.d/",
        };
        // here the logger is not yet available, so print it to stdout
        println!("Using rules_dir directory: [{}]", rules_dir);
        rules_dir
    }

    pub fn drafts_dir(&self) -> &str {
        let drafts_dir = match &self.drafts_dir {
            Some(drafts_dir) => drafts_dir,
            None => "/drafts/",
        };
        // here the logger is not yet available, so print it to stdout
        println!("Using drafts_dir directory: [{}]", drafts_dir);
        drafts_dir
    }
}

#[derive(Clap, Debug)]
pub enum SubCommand {
    /// Checks that the configuration is valid
    Check,

    /// Starts the Tornado daemon
    Daemon,

    /// Starts the Tornado Rules upgrade process
    RulesUpgrade,

    /// Enable or disable the APM logger priority configuration.
    /// When used with `enable`, it:
    /// - enables the elastic-APM logger output
    /// - disables the stdout logger output
    /// - increases the logger level to debug
    ///
    /// When used with `disable`, it:
    /// - sets the logger level back to original value from the configuration file
    /// - enables the stdout logger output
    /// - disables the elastic-APM logger output
    ///
    ApmTracing {
        #[clap(subcommand)]
        command: EnableOrDisableSubCommand,
    },

}

#[derive(Clap, Debug)]
pub enum EnableOrDisableSubCommand {
    Enable,
    Disable
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[allow(clippy::upper_case_acronyms)]
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
    #[serde(default)]
    pub nats_extractors: Vec<NatsExtractor>,

    pub web_server_ip: String,
    pub web_server_port: u16,
    pub web_max_json_payload_size: Option<usize>,

    pub message_queue_size: usize,

    pub thread_pool_config: Option<ThreadPoolConfig>,
    #[serde(default)]
    pub retry_strategy: RetryStrategy,

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

fn build_director_client_config(config_dir: &str) -> Result<DirectorClientConfig, ConfigError> {
    let config_file_path = format!("{}/director_client_executor.toml", config_dir);
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

fn build_smart_monitoring_check_result_config(
    config_dir: &str,
) -> Result<SmartMonitoringCheckResultConfig, ConfigError> {
    let config_file_path = format!("{}/smart_monitoring_check_result.toml", config_dir);
    if Path::new(&config_file_path).exists() {
        let mut s = Config::new();
        s.merge(File::with_name(&config_file_path))?;
        s.try_into()
    } else {
        warn!(
            "Cannot find configuration file [{}]. The default config will be used.",
            config_file_path
        );
        Ok(Default::default())
    }
}

pub struct ComponentsConfig {
    pub matcher_config: Arc<FsMatcherConfigManager>,
    pub archive_executor_config: ArchiveConfig,
    pub icinga2_executor_config: Icinga2ClientConfig,
    pub director_executor_config: DirectorClientConfig,
    pub elasticsearch_executor_config: ElasticsearchConfig,
    pub smart_monitoring_check_result_config: SmartMonitoringCheckResultConfig,
}

pub fn parse_config_files(
    config_dir: &str,
    rules_dir: &str,
    drafts_dir: &str,
) -> Result<ComponentsConfig, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let matcher_config = Arc::new(build_matcher_config(config_dir, rules_dir, drafts_dir));
    let archive_executor_config = build_archive_config(config_dir)?;
    let icinga2_executor_config = build_icinga2_client_config(config_dir)?;
    let director_executor_config = build_director_client_config(config_dir)?;
    let elasticsearch_executor_config = build_elasticsearch_config(config_dir)?;
    let smart_monitoring_check_result_config =
        build_smart_monitoring_check_result_config(config_dir)?;
    Ok(ComponentsConfig {
        matcher_config,
        archive_executor_config,
        icinga2_executor_config,
        director_executor_config,
        elasticsearch_executor_config,
        smart_monitoring_check_result_config,
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
            vec![
                Permission::ConfigEdit,
                Permission::ConfigView,
                Permission::RuntimeConfigEdit,
                Permission::RuntimeConfigView
            ],
            config.tornado.daemon.auth.role_permissions["admin"]
        );

    }

    #[tokio::test]
    async fn should_read_all_rule_configurations_from_file() {
        // Arrange
        let path = "./config/rules.d";
        let drafts_path = "./config/drafts";

        // Act
        let config = FsMatcherConfigManager::new(path, drafts_path).get_config().await.unwrap();

        // Assert
        match config {
            MatcherConfig::Filter { name, nodes, .. } => {
                assert_eq!("root", name);
                assert_eq!(2, nodes.len());
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
            "https://localhost:5665/v1/actions",
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
        assert_eq!("https://localhost:5665/v1/actions", config.server_api_url)
    }

    #[test]
    fn should_read_director_client_configurations_from_file() {
        // Arrange
        let config_dir = "./config";

        // Act
        let config = build_director_client_config(config_dir).unwrap();

        // Assert
        assert_eq!("https://localhost/neteye/director", config.server_api_url)
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
            nats_extractors: vec![],
            web_server_ip: "".to_string(),
            web_server_port: 0,
            web_max_json_payload_size: None,
            message_queue_size: 0,
            thread_pool_config: None,
            retry_strategy: Default::default(),
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
            nats_extractors: vec![],
            web_server_ip: "".to_string(),
            web_server_port: 0,
            web_max_json_payload_size: None,
            message_queue_size: 0,
            thread_pool_config: None,
            retry_strategy: Default::default(),
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
        let random = match rand::random::<isize>().abs() {
            0 => 1,
            x => x,
        };
        assert_eq!(random as usize, ThreadPoolConfig::Fixed { size: random }.get_threads_count());
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
