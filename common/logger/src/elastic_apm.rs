use crate::LoggerError;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use tracing_elastic_apm::config::ApiKey;

pub const DEFAULT_APM_SERVER_CREDENTIALS_FILENAME: &str = "apm_server_api_credentials.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApmTracingConfig {
    // The url of the Elastic APM server; if provided, traces will be sent to this server;
    // if not provided traces will not be sent.
    pub apm_server_url: Option<String>,

    // The path of file containing credentials for calling the APM server APIs;
    pub apm_server_credentials_filepath: Option<String>,
}

impl Default for ApmTracingConfig {
    fn default() -> Self {
        Self { apm_server_url: None, apm_server_credentials_filepath: None }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ApiCredentials {
    id: String,
    key: String,
}

impl From<ApiCredentials> for ApiKey {
    fn from(api_credentials: ApiCredentials) -> Self {
        ApiKey::new(api_credentials.id, api_credentials.key)
    }
}

impl ApiCredentials {
    pub fn from_file(apm_server_credentials_filepath: &str) -> Result<Self, LoggerError> {
        let apm_server_credentials_file = File::open(&apm_server_credentials_filepath)?;
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
            ApiCredentials::from_file("./test_resources/apm_server_api_credentials.json").unwrap();
        assert_eq!(
            api_credentials,
            ApiCredentials { id: "myid".to_string(), key: "mykey".to_string() }
        );
    }

    #[test]
    fn should_read_api_credentials_should_return_error_if_file_does_not_exist() {
        let res = ApiCredentials::from_file("./non-existing.json");
        assert!(res.is_err());
    }

    #[test]
    fn should_read_api_credentials_should_return_error_if_file_is_not_correcly_formatted() {
        let res =
            ApiCredentials::from_file("./test_resources/apm_server_api_credentials_wrong.json");
        assert!(res.is_err());
    }
}
