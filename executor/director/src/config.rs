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
pub struct DirectorClient {
    pub server_api_url: String,
    pub http_auth_header: String,
    pub client: Client,
}

#[derive(Deserialize)]
pub struct Icinga2RestartCurrentStatus {
    pub pending: bool,
}

impl DirectorClient {
    pub async fn get_icinga2_restart_current_status(
        &self,
    ) -> Result<Icinga2RestartCurrentStatus, ExecutorError> {
        let url = format!("{}/icinga2restart/currentstatus", self.server_api_url);
        match self
            .client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::AUTHORIZATION, self.http_auth_header.as_str())
            .send()
            .await
        {
            Ok(res) => {
                if res.status().is_success() {
                    res.json().await.map_err(|err| ExecutorError::RuntimeError {
                        message: format!(
                            "Failed to get the current status of the Icinga 2 restart. Err: {}",
                            err
                        ),
                    })
                } else {
                    Err(ExecutorError::RuntimeError { message: format!("Failed to get the current status of the Icinga 2 restart. API status code: {}", res.status()) })
                }
            }
            Err(err) => Err(ExecutorError::RuntimeError {
                message: format!(
                    "Failed to get the current status of the Icinga 2 restart. Err: {}",
                    err
                ),
            }),
        }
    }
}

impl DirectorClientConfig {
    pub fn new_client(&self) -> Result<DirectorClient, ExecutorError> {
        let auth = format!("{}:{}", self.username, self.password);
        let http_auth_header = format!("Basic {}", base64::encode(&auth));

        let mut client_builder = Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(self.timeout_secs.unwrap_or(10)));

        if self.disable_ssl_verification {
            client_builder = client_builder.danger_accept_invalid_certs(true)
        }

        let client = client_builder.build().map_err(|err| ExecutorError::ConfigurationError {
            message: format!("Error while building DirectorClient. Err: {:?}", err),
        })?;

        Ok(DirectorClient { server_api_url: self.server_api_url.clone(), http_auth_header, client })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use httpmock::Method::GET;
    use httpmock::MockServer;

    #[tokio::test]
    async fn should_get_icinga2_restart_current_status() {
        // Arrange
        let mock_server = MockServer::start();

        mock_server.mock(|when, then| {
            when.method(GET).path("/icinga2restart/currentstatus");
            then.status(200).body("{\"pending\": true}");
        });
        let director_port = mock_server.port();

        let client = DirectorClientConfig {
            server_api_url: format!("http://localhost:{}", director_port),
            username: "".to_string(),
            password: "".to_string(),
            disable_ssl_verification: false,
            timeout_secs: None,
        }
        .new_client()
        .unwrap();

        // Act
        let result = client.get_icinga2_restart_current_status().await;

        // Assert
        assert_eq!(result.unwrap().pending, true);
    }

    #[tokio::test]
    async fn get_icinga2_restart_current_status_should_fail_if_not_succeeded() {
        // Arrange
        let mock_server = MockServer::start();

        mock_server.mock(|when, then| {
            when.method(GET).path("/icinga2restart/currentstatus");
            then.status(400);
        });
        let director_port = mock_server.port();

        let client = DirectorClientConfig {
            server_api_url: format!("http://localhost:{}", director_port),
            username: "".to_string(),
            password: "".to_string(),
            disable_ssl_verification: false,
            timeout_secs: None,
        }
        .new_client()
        .unwrap();

        // Act
        let result = client.get_icinga2_restart_current_status().await;

        // Assert
        assert!(result.is_err());
    }
}
