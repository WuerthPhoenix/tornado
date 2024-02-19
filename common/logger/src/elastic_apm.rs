use crate::LoggerError;
use base64::{engine::general_purpose::STANDARD as base64, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

pub const DEFAULT_APM_SERVER_CREDENTIALS_FILENAME: &str = "apm_server_api_credentials.json";
pub const DEFAULT_EXPORTER_BATCH_SIZE: u64 = 65536;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApmTracingConfig {
    // Whether the Logger data should be sent to the Elastic APM Server.
    pub apm_output: bool,

    // The url of the Elastic APM server.
    pub apm_server_url: String,

    // The credentials for calling the APM server APIs;
    pub apm_server_api_credentials: Option<ApmServerApiCredentials>,

    #[serde(default)]
    pub exporter: ExporterConfig,
}

// These settings could be configured via env variables,
// but to have the configuration centralized and to be
// consistent with the other Tornado settings
// we deserialize them from the config.
// More info for the settings can be found here: https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/sdk-environment-variables.md#batch-span-processor
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExporterConfig {
    // Span exporter queue size
    pub max_queue_size: u64,

    // The delay interval in milliseconds between two consecutive processing
    // of batches.
    pub scheduled_delay_ms: Option<u64>,

    // Span exporter batch size
    pub max_export_batch_size: Option<u64>,

    // The maximum duration to export a batch of data.
    pub max_export_timeout_ms: Option<u64>,
}

impl Default for ExporterConfig {
    fn default() -> Self {
        Self {
            max_queue_size: DEFAULT_EXPORTER_BATCH_SIZE,
            scheduled_delay_ms: None,
            max_export_batch_size: None,
            max_export_timeout_ms: None,
        }
    }
}

impl ApmTracingConfig {
    pub fn read_apm_server_api_credentials_if_not_set(
        &mut self,
        filepath: &str,
    ) -> Result<(), LoggerError> {
        if self.apm_server_api_credentials.is_none() {
            self.apm_server_api_credentials =
                Some(ApmServerApiCredentials::from_file(filepath).map_err(|err| {
                    LoggerError::LoggerConfigurationError {
                        message: format!(
                            "Could not set APM Server credentials from file '{}'. Error: {:?}",
                            filepath, err
                        ),
                    }
                })?);
        }
        Ok(())
    }
}

impl Default for ApmTracingConfig {
    fn default() -> Self {
        Self {
            apm_output: false,
            apm_server_url: "http://localhost:8200".to_string(),
            apm_server_api_credentials: None,
            exporter: ExporterConfig::default(),
        }
    }
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct ApmServerApiCredentials {
    pub id: String,
    pub key: String,
}

impl ApmServerApiCredentials {
    pub fn from_file(apm_server_credentials_filepath: &str) -> Result<Self, LoggerError> {
        let apm_server_credentials_file = File::open(apm_server_credentials_filepath)?;
        let apm_server_credentials_reader = BufReader::new(apm_server_credentials_file);

        serde_json::from_reader(apm_server_credentials_reader).map_err(|err| {
            LoggerError::LoggerConfigurationError {
                message: format!(
                    "Failed to read APM server Api Key from file {}. Error: {:?}",
                    &apm_server_credentials_filepath, err
                ),
            }
        })
    }

    /// Returns the ApiKey "authorization" header value constructed as of Elastic specifications:
    /// https://www.elastic.co/guide/en/apm/server/current/api-key.html#create-api-key-workflow-es
    /// This can be used in the "authorization" HTTP header to authenticate to APM Server, and
    /// other Elastic services.
    pub fn to_authorization_header_value(&self) -> String {
        format!("ApiKey {}", base64.encode(format!("{}:{}", self.id, self.key)))
    }
}

pub fn get_current_service_name() -> Result<String, LoggerError> {
    let current_executable = std::env::current_exe()?;
    let filename = current_executable
        .file_name()
        .and_then(|filename_os_str| filename_os_str.to_str())
        .map(|filename_str| filename_str.to_string());
    filename.ok_or(LoggerError::LoggerRuntimeError {
        message: "Could not get current executable file name".to_string(),
    })
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_get_correct_service_name() {
        assert!(get_current_service_name().unwrap().starts_with("tornado_common_logger"));
    }

    #[test]
    fn should_read_api_credentials_correct_file() {
        let api_credentials =
            ApmServerApiCredentials::from_file("./test_resources/apm_server_api_credentials.json")
                .unwrap();
        assert_eq!(
            api_credentials,
            ApmServerApiCredentials { id: "myid".to_string(), key: "mykey".to_string() }
        );
    }

    #[test]
    fn should_read_api_credentials_should_return_error_if_file_does_not_exist() {
        let res = ApmServerApiCredentials::from_file("./non-existing.json");
        assert!(res.is_err());
    }

    #[test]
    fn should_read_api_credentials_should_return_error_if_file_is_not_correcly_formatted() {
        let res = ApmServerApiCredentials::from_file(
            "./test_resources/apm_server_api_credentials_wrong.json",
        );
        assert!(res.is_err());
    }

    #[test]
    fn should_construct_authorization_header_from_api_credentials() {
        let creds = ApmServerApiCredentials {
            id: "GnrUT3QB7yZbSNxKET6d".to_string(),
            key: "RhHKisTmQ1aPCHC_TPwOvw".to_string(),
        };
        assert_eq!(
            creds.to_authorization_header_value(),
            "ApiKey R25yVVQzUUI3eVpiU054S0VUNmQ6UmhIS2lzVG1RMWFQQ0hDX1RQd092dw=="
        );
    }

    #[test]
    fn should_deserialize_apm_tracing_config() {
        // Arrange
        let expected_config = ApmTracingConfig {
            apm_output: true,
            apm_server_url: "".to_string(),
            apm_server_api_credentials: None,
            exporter: ExporterConfig {
                max_queue_size: 65536,
                scheduled_delay_ms: None,
                max_export_batch_size: None,
                max_export_timeout_ms: None,
            },
        };

        // Act
        let tracing_config: ApmTracingConfig = serde_json::from_str(
            r#"
        {
          "apm_output": true,
          "apm_server_url": ""
        }"#,
        )
        .unwrap();

        // Assert
        assert_eq!(tracing_config, expected_config);
    }
}
