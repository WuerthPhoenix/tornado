use crate::config::{ApiClient, Icinga2ClientConfig};
use log::*;
use serde::Serialize;
use tornado_common_api::Action;
use tornado_common_api::Payload;
use tornado_executor_common::{Executor, ExecutorError};

pub mod config;

pub const ICINGA2_ACTION_NAME_KEY: &str = "icinga2_action_name";
pub const ICINGA2_ACTION_PAYLOAD_KEY: &str = "icinga2_action_payload";

const ICINGA2_OBJECT_NOT_EXISTING_RESPONSE: &str = "No objects found";
const ICINGA2_OBJECT_NOT_EXISTING_STATUS_CODE: u16 = 404;
pub const ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE: &str = "IcingaObjectNotExisting";

/// An executor that logs received actions at the 'info' level
pub struct Icinga2Executor {
    api_client: ApiClient,
}

impl std::fmt::Display for Icinga2Executor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("Icinga2Executor")?;
        Ok(())
    }
}

impl Icinga2Executor {
    pub fn new(config: Icinga2ClientConfig) -> Result<Icinga2Executor, ExecutorError> {
        Ok(Icinga2Executor { api_client: config.new_client()? })
    }

    fn get_payload<'a>(&self, payload: &'a Payload) -> Option<&'a Payload> {
        payload.get(ICINGA2_ACTION_PAYLOAD_KEY).and_then(tornado_common_api::Value::get_map)
    }

    fn parse_action<'a>(&self, action: &'a Action) -> Result<Icinga2Action<'a>, ExecutorError> {
        match action
            .payload
            .get(ICINGA2_ACTION_NAME_KEY)
            .and_then(tornado_common_api::Value::get_text)
        {
            Some(icinga2_action) => {
                trace!("Icinga2Executor - perform Icinga2Action: \n[{:?}]", icinga2_action);

                let action_payload = self.get_payload(&action.payload);

                Ok(Icinga2Action { name: icinga2_action, payload: action_payload })
            }
            None => Err(ExecutorError::MissingArgumentError {
                message: "Icinga2 Action not specified".to_string(),
            }),
        }
    }

    pub fn perform_request(&self, icinga2_action: &Icinga2Action) -> Result<(), ExecutorError> {
        let url = format!("{}/{}", &self.api_client.server_api_url, icinga2_action.name);
        let http_auth_header = &self.api_client.http_auth_header;
        let client = &self.api_client.client;

        trace!("Icinga2Executor - calling url: {}", url);

        let mut response = client
            .post(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::AUTHORIZATION, http_auth_header)
            .json(&icinga2_action.payload)
            .send()
            .map_err(|err| ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!("Icinga2Executor - Connection failed. Err: {}", err),
                code: None,
            })?;

        let response_status = response.status();

        let response_body = response.text().map_err(|err| ExecutorError::ActionExecutionError {
            can_retry: true,
            message: format!("Icinga2Executor - Cannot extract response body. Err: {}", err),
            code: None,
        })?;

        if response_status.eq(&ICINGA2_OBJECT_NOT_EXISTING_STATUS_CODE)
            && response_body.contains(ICINGA2_OBJECT_NOT_EXISTING_RESPONSE)
        {
            Err(ExecutorError::ActionExecutionError { message: format!("Icinga2Executor - Icinga2 API returned an error, object seems to be not existing in Icinga2. Response status: {}. Response body: {}", response_status, response_body ), can_retry: true, code: Some(ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE) })
        } else if !response_status.is_success() {
            Err(ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!(
                    "Icinga2Executor - Icinga2 API returned an error. Response status: {}. Response body: {}", response_status, response_body
                ),
                code: None
            })
        } else {
            debug!("Icinga2Executor - Data correctly sent to Icinga2 API");
            Ok(())
        }
    }
}

impl Executor for Icinga2Executor {
    fn execute(&self, action: &Action) -> Result<(), ExecutorError> {
        trace!("Icinga2Executor - received action: \n[{:?}]", action);
        let action = self.parse_action(action)?;

        self.perform_request(&action)
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Icinga2Action<'a> {
    pub name: &'a str,
    pub payload: Option<&'a Payload>,
}

#[cfg(test)]
mod test {
    use super::*;
    use maplit::*;
    use tornado_common_api::Value;

    #[test]
    fn should_fail_if_action_missing() {
        // Arrange

        let executor = Icinga2Executor::new(Icinga2ClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: "".to_owned(),
        })
        .unwrap();

        let action = Action::new("");

        // Act
        let result = executor.parse_action(&action);

        // Assert
        assert!(result.is_err());
        assert_eq!(
            Err(ExecutorError::MissingArgumentError {
                message: "Icinga2 Action not specified".to_owned()
            }),
            result
        );
    }

    #[test]
    fn should_have_empty_payload_if_action_does_not_contains_one() {
        // Arrange
        let executor = Icinga2Executor::new(Icinga2ClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: "".to_owned(),
        })
        .unwrap();

        let mut action = Action::new("");
        action
            .payload
            .insert(ICINGA2_ACTION_NAME_KEY.to_owned(), Value::Text("action-test".to_owned()));

        // Act
        let result = executor.parse_action(&action);

        // Assert
        assert_eq!(Ok(Icinga2Action { name: "action-test", payload: None }), result);
    }

    #[test]
    fn should_parse_valid_action() {
        // Arrange
        let executor = Icinga2Executor::new(Icinga2ClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: "".to_owned(),
        })
        .unwrap();

        let mut action = Action::new("");
        action.payload.insert(
            ICINGA2_ACTION_NAME_KEY.to_owned(),
            Value::Text("process-check-result".to_owned()),
        );
        action.payload.insert(
            ICINGA2_ACTION_PAYLOAD_KEY.to_owned(),
            Value::Map(hashmap![
                "filter".to_owned() => Value::Text("filter_value".to_owned()),
                "type".to_owned() => Value::Text("Host".to_owned())
            ]),
        );

        // Act
        let result = executor.parse_action(&action);

        // Assert
        assert_eq!(
            Ok(Icinga2Action {
                name: "process-check-result",
                payload: Some(&hashmap![
                    "filter".to_owned() => Value::Text("filter_value".to_owned()),
                    "type".to_owned() => Value::Text("Host".to_owned())
                ])
            }),
            result
        );
    }
}
