use crate::config::Icinga2ClientConfig;
use log::*;
use reqwest::{Client, Response};
use serde::Serialize;
use std::time::Duration;
use tornado_executor_common::ExecutorError;

#[derive(Clone)]
pub struct ApiClient {
    pub server_api_url: String,
    pub http_auth_header: String,
    client: Client,
}

impl ApiClient {
    pub fn new(config: &Icinga2ClientConfig) -> Result<ApiClient, ExecutorError> {
        let auth = format!("{}:{}", config.username, config.password);
        let http_auth_header = format!("Basic {}", base64::encode(&auth));

        let mut client_builder = Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(config.timeout_secs.unwrap_or(10)));

        if config.disable_ssl_verification {
            client_builder = client_builder.danger_accept_invalid_certs(true)
        }

        let client = client_builder.build().map_err(|err| ExecutorError::ConfigurationError {
            message: format!("Error while building Icinga2Client. Err: {}", err),
        })?;

        // The server API url should not contain the /v1/actions suffix.
        // Clean the URL as users have this suffix in their configuration.
        let mut server_api_url = config.server_api_url.replace("/v1/actions", "");
        if server_api_url.ends_with('/') {
            server_api_url = server_api_url[0..server_api_url.len()-1].to_owned()
        }

        Ok(ApiClient { server_api_url, http_auth_header, client })
    }

    fn post<T: Serialize + ?Sized>(
        &self,
        icinga2_api_name: &str,
        payload: &T,
    ) -> Result<Response, ExecutorError> {
        let url = format!("{}{}", &self.server_api_url, icinga2_api_name);
        let http_auth_header = &self.http_auth_header;

        trace!("Icinga2Executor - HTTP POST - url: {}", url);

        self.client
            .post(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::AUTHORIZATION, http_auth_header)
            .json(payload)
            .send()
            .map_err(|err| ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!("Icinga2Executor - Connection failed. Err: {}", err),
                code: None,
            })
    }

    fn get(&self, icinga2_api_name: &str) -> Result<Response, ExecutorError> {
        let url = format!("{}{}", &self.server_api_url, icinga2_api_name);
        let http_auth_header = &self.http_auth_header;

        trace!("Icinga2Executor - HTTP GET - url: {}", url);

        self.client
            .get(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::AUTHORIZATION, http_auth_header)
            .send()
            .map_err(|err| ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!("Icinga2Executor - Connection failed. Err: {}", err),
                code: None,
            })
    }

    pub fn api_get_objects_host(&self, host_name: &str) -> Result<Response, ExecutorError> {
        self.get(&format!("/v1/objects/hosts/{}", host_name))
    }

    pub fn api_get_objects_service(
        &self,
        host_name: &str,
        service_name: &str,
    ) -> Result<Response, ExecutorError> {
        self.get(&format!("/v1/objects/services/{}!{}", host_name, service_name))
    }

    pub fn api_post_action<T: Serialize + ?Sized>(
        &self,
        icinga2_action_name: &str,
        payload: &T,
    ) -> Result<Response, ExecutorError> {
        let url = format!("/v1/actions/{}", icinga2_action_name);
        self.post(&url, payload)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_remove_actions_suffix_from_url() {
        // Arrange
        let mut config = Icinga2ClientConfig {
            username: "".to_owned(),
            disable_ssl_verification: false,
            password: "".to_owned(),
            timeout_secs: None,
            server_api_url: "http://localhost".to_owned()
        };

        // Act & Assert
        assert_eq!( "http://localhost", ApiClient::new(&config).unwrap().server_api_url);

        {
            let url = "http://localhost:8080/";
            config.server_api_url = url.to_owned();
            assert_eq!( "http://localhost:8080", ApiClient::new(&config).unwrap().server_api_url);
        }

        {
            let url = "http://localhost:8080/v1/actions";
            config.server_api_url = url.to_owned();
            assert_eq!( "http://localhost:8080" , ApiClient::new(&config).unwrap().server_api_url);
        }

        {
            let url = "http://127.0.0.1:8080/v1/actions/";
            config.server_api_url = url.to_owned();
            assert_eq!( "http://127.0.0.1:8080" , ApiClient::new(&config).unwrap().server_api_url);
        }
    }
}