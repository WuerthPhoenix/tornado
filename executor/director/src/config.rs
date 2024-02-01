use base64::{engine::general_purpose::STANDARD as base64, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tornado_executor_common::ExecutorError;

#[derive(Deserialize, Serialize, Clone)]
pub struct DirectorClientConfig {
    /// The complete URL of the API Server
    pub server_api_url: String,

    /// Username used to connect to the APIs
    pub username: String,

    /// Password used to connect to the APIs
    pub password: String,

    /// If true, the client will not verify the SSL certificate
    pub disable_ssl_verification: bool,

    /// The call timeout in seconds. Default is 10 seconds
    pub timeout_secs: Option<u64>,
}

#[derive(Clone)]
pub struct ApiClient {
    pub server_api_url: String,
    pub http_auth_header: String,
    pub client: Client,
}

impl DirectorClientConfig {
    pub fn new_client(&self) -> Result<ApiClient, ExecutorError> {
        let auth = format!("{}:{}", self.username, self.password);
        let http_auth_header = format!("Basic {}", base64.encode(auth));

        let mut client_builder = Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(self.timeout_secs.unwrap_or(10)));

        if self.disable_ssl_verification {
            client_builder = client_builder.danger_accept_invalid_certs(true)
        }

        let client = client_builder.build().map_err(|err| ExecutorError::ConfigurationError {
            message: format!("Error while building DirectorClient. Err: {:?}", err),
        })?;

        Ok(ApiClient { server_api_url: self.server_api_url.clone(), http_auth_header, client })
    }
}
