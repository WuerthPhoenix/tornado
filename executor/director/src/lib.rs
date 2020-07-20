use crate::config::{ApiClient, DirectorClientConfig};
use log::*;
use serde::*;
use tornado_common_api::Action;
use tornado_common_api::Payload;
use tornado_common_api::Value;
use tornado_executor_common::{Executor, ExecutorError};

pub mod config;

pub const DIRECTOR_ACTION_NAME_KEY: &str = "action_name";
pub const DIRECTOR_ACTION_PAYLOAD_KEY: &str = "action_payload";
pub const DIRECTOR_ACTION_LIVE_CREATION_KEY: &str = "icinga2_live_creation";

const ICINGA2_OBJECT_ALREADY_EXISTING_STATUS_CODE: u16 = 422;
const ICINGA2_OBJECT_ALREADY_EXISTING_RESPONSE: &str = "Trying to recreate";
pub const ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE: &str = "IcingaObjectAlreadyExisting";

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum DirectorActionName {
    CreateHost,
    CreateService,
}

impl DirectorActionName {
    fn from_str(name: &str) -> Result<Self, ExecutorError> {
        match name {
            "create_host" => Ok(DirectorActionName::CreateHost),
            "create_service" => Ok(DirectorActionName::CreateService),
            val => Err(ExecutorError::UnknownArgumentError { message: format!("Invalid action_name value. Found: '{}'. Expected valid action_name. Refer to the documentation",val) })
        }
    }

    pub fn to_director_api_subpath(&self) -> &str {
        match self {
            DirectorActionName::CreateHost => "host",
            DirectorActionName::CreateService => "service",
        }
    }
}

/// An executor that calls the APIs of the IcingaWeb2 Director
pub struct DirectorExecutor {
    api_client: ApiClient,
}

impl std::fmt::Display for DirectorExecutor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("DirectorExecutor")?;
        Ok(())
    }
}

impl DirectorExecutor {
    pub fn new(config: DirectorClientConfig) -> Result<DirectorExecutor, ExecutorError> {
        Ok(DirectorExecutor { api_client: config.new_client()? })
    }

    fn get_payload<'a>(&self, payload: &'a Payload) -> Result<&'a Value, ExecutorError> {
        payload.get(DIRECTOR_ACTION_PAYLOAD_KEY).ok_or(ExecutorError::MissingArgumentError {
            message: "Director Action Payload not specified".to_string(),
        })
    }

    fn get_live_creation_setting(&self, payload: &Payload) -> bool {
        payload
            .get(DIRECTOR_ACTION_LIVE_CREATION_KEY)
            .and_then(tornado_common_api::Value::get_bool)
            .unwrap_or(&false)
            .to_owned()
    }

    fn parse_action<'a>(
        &mut self,
        action: &'a Action,
    ) -> Result<DirectorAction<'a>, ExecutorError> {
        let director_action_name = action
            .payload
            .get(DIRECTOR_ACTION_NAME_KEY)
            .and_then(tornado_common_api::Value::get_text)
            .ok_or(ExecutorError::MissingArgumentError {
                message: "Director Action not specified".to_string(),
            })
            .and_then(DirectorActionName::from_str)?;

        trace!("DirectorExecutor - perform DirectorAction: \n[{:?}]", director_action_name);

        let action_payload = self.get_payload(&action.payload)?;

        let live_creation = self.get_live_creation_setting(&action.payload);

        Ok(DirectorAction { name: director_action_name, payload: action_payload, live_creation })
    }

    pub fn perform_request(&self, director_action: DirectorAction) -> Result<(), ExecutorError> {
        let mut url = format!(
            "{}/{}",
            &self.api_client.server_api_url,
            director_action.name.to_director_api_subpath()
        );

        trace!(
            "DirectorExecutor - icinga2 live creation is set to: {}",
            director_action.live_creation
        );
        if director_action.live_creation {
            url.push_str("?live-creation=true");
        }
        let http_auth_header = &self.api_client.http_auth_header;
        let client = &self.api_client.client;

        trace!("DirectorExecutor - calling url: {}", url);

        let mut response = client
            .post(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::AUTHORIZATION, http_auth_header.as_str())
            .json(&director_action.payload)
            .send()
            .map_err(|err| ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!("DirectorExecutor - Connection failed. Err: {:?}", err),
                code: None,
            })?;

        let response_status = response.status();

        let response_body = response.text().map_err(|err| ExecutorError::ActionExecutionError {
            can_retry: true,
            message: format!("DirectorExecutor - Cannot extract response body. Err: {:?}", err),
            code: None,
        })?;

        if response_status.eq(&ICINGA2_OBJECT_ALREADY_EXISTING_STATUS_CODE)
            && response_body.contains(ICINGA2_OBJECT_ALREADY_EXISTING_RESPONSE)
        {
            Err(ExecutorError::ActionExecutionError { message: format!("DirectorExecutor - Icinga Director API returned an error, object seems to be already existing. Response status: {}. Response body: {}", response_status, response_body ), can_retry: true, code: Some(ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE) })
        } else if !response_status.is_success() {
            Err(ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!(
                    "DirectorExecutor API returned an error. Response status: {}. Response body: {}", response_status, response_body
                ),
                code: None
            })
        } else {
            debug!(
                "DirectorExecutor API request completed successfully. Response body: {}",
                response_body
            );
            Ok(())
        }
    }
}

impl Executor for DirectorExecutor {
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError> {
        trace!("DirectorExecutor - received action: \n[{:?}]", action);

        let action = self.parse_action(action)?;

        self.perform_request(action)
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct DirectorAction<'a> {
    pub name: DirectorActionName,
    pub payload: &'a Value,
    pub live_creation: bool,
}

#[cfg(test)]
mod test {
    use super::*;
    use maplit::*;
    use tornado_common_api::Value;

    #[test]
    fn should_fail_if_action_missing() {
        // Arrange
        let mut executor = DirectorExecutor::new(DirectorClientConfig {
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
        assert_eq!(
            Err(ExecutorError::MissingArgumentError {
                message: "Director Action not specified".to_owned()
            }),
            result
        );
    }

    #[test]
    fn should_throw_error_if_action_payload_is_not_set() {
        // Arrange
        let mut executor = DirectorExecutor::new(DirectorClientConfig {
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
            .insert(DIRECTOR_ACTION_NAME_KEY.to_owned(), Value::Text("create_service".to_owned()));
        action.payload.insert(DIRECTOR_ACTION_LIVE_CREATION_KEY.to_owned(), Value::Bool(true));

        // Act
        let result = executor.parse_action(&action);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_parse_valid_action() {
        // Arrange
        let mut executor = DirectorExecutor::new(DirectorClientConfig {
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
            .insert(DIRECTOR_ACTION_NAME_KEY.to_owned(), Value::Text("create_host".to_owned()));
        action.payload.insert(
            DIRECTOR_ACTION_PAYLOAD_KEY.to_owned(),
            Value::Map(hashmap![
                "filter".to_owned() => Value::Text("filter_value".to_owned()),
                "type".to_owned() => Value::Text("Host".to_owned())
            ]),
        );

        // Act
        let result = executor.parse_action(&action);

        // Assert
        assert_eq!(
            Ok(DirectorAction {
                name: DirectorActionName::CreateHost,
                payload: &Value::Map(hashmap![
                    "filter".to_owned() => Value::Text("filter_value".to_owned()),
                    "type".to_owned() => Value::Text("Host".to_owned())
                ]),
                live_creation: false
            }),
            result
        );
    }
}
