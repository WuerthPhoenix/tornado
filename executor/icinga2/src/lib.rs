use crate::client::ApiClient;
use crate::config::Icinga2ClientConfig;
use log::*;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use tornado_common_api::Payload;
use tornado_common_api::{Action, TracedAction};
use tornado_executor_common::{ExecutorError, StatelessExecutor};

pub mod client;
pub mod config;

pub const ICINGA2_ACTION_NAME_KEY: &str = "icinga2_action_name";
pub const ICINGA2_ACTION_PAYLOAD_KEY: &str = "icinga2_action_payload";

const ICINGA2_OBJECT_NOT_EXISTING_RESPONSE: &str = "No objects found";
const ICINGA2_OBJECT_NOT_EXISTING_STATUS_CODE: u16 = 404;
pub const ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE: &str = "IcingaObjectNotExisting";

/// An executor that logs received actions at the 'info' level
#[derive(Clone)]
pub struct Icinga2Executor {
    pub api_client: ApiClient,
}

impl std::fmt::Display for Icinga2Executor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("Icinga2Executor")?;
        Ok(())
    }
}

impl Icinga2Executor {
    pub fn new(config: Icinga2ClientConfig) -> Result<Icinga2Executor, ExecutorError> {
        Ok(Icinga2Executor { api_client: ApiClient::new(&config)? })
    }

    fn get_payload<'a>(&self, payload: &'a Payload) -> Option<&'a Payload> {
        payload.get(ICINGA2_ACTION_PAYLOAD_KEY).and_then(tornado_common_api::ValueExt::get_map)
    }

    fn parse_action<'a>(&self, action: &'a Action) -> Result<Icinga2Action<'a>, ExecutorError> {
        match action
            .payload
            .get(ICINGA2_ACTION_NAME_KEY)
            .and_then(tornado_common_api::ValueExt::get_text)
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

    pub async fn perform_request<'a>(
        &self,
        icinga2_action: &'a Icinga2Action<'a>,
    ) -> Result<(), ExecutorError> {
        let payload = &icinga2_action.payload;
        let response = self.api_client.api_post_action(icinga2_action.name, payload).await?;

        let method = response.method;
        let url = response.url;

        let response_status = response.response.status();

        let response_body = response.response.text().await.map_err(|err| {
            match to_err_data(method, &url, payload) {
                Ok(data) => ExecutorError::ActionExecutionError {
                    can_retry: true,
                    message: format!(
                        "Icinga2Executor - Cannot extract response body. Err: {:?}",
                        err
                    ),
                    code: None,
                    data: data.into(),
                },
                Err(err) => err,
            }
        })?;

        if response_status.eq(&ICINGA2_OBJECT_NOT_EXISTING_STATUS_CODE)
            && response_body.contains(ICINGA2_OBJECT_NOT_EXISTING_RESPONSE)
        {
            Err(ExecutorError::ActionExecutionError {
                message: format!("Icinga2Executor - Icinga2 API returned an error, object seems to be not existing in Icinga2. Response status: {}. Response body: {}", response_status, response_body ),
                can_retry: true,
                code: Some(ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE),
                data: to_err_data(method, &url, payload)?.into()
            })
        } else if !response_status.is_success() {
            Err(ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!(
                    "Icinga2Executor - Icinga2 API returned an error. Response status: {}. Response body: {}", response_status, response_body
                ),
                code: None,
                data: to_err_data(method, &url, payload)?.into()
            })
        } else {
            debug!("Icinga2Executor - Data correctly sent to Icinga2 API");
            Ok(())
        }
    }
}

fn to_err_data(
    method: &str,
    url: &str,
    payload: &Option<&Payload>,
) -> Result<HashMap<&'static str, Value>, ExecutorError> {
    let mut data = HashMap::<&'static str, Value>::default();
    data.insert("method", method.into());
    data.insert("url", url.into());
    data.insert("payload", serde_json::to_value(payload)?);
    Ok(data)
}

#[async_trait::async_trait(?Send)]
impl StatelessExecutor for Icinga2Executor {
    async fn execute(&self, action: TracedAction) -> Result<(), ExecutorError> {
        trace!("Icinga2Executor - received action: \n[{:?}]", action);
        let action = self.parse_action(&action.action)?;

        self.perform_request(&action).await
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
    use serde_json::json;
    use tornado_common_api::{Map, Value};

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
            .insert(ICINGA2_ACTION_NAME_KEY.to_owned(), Value::String("action-test".to_owned()));

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
            Value::String("process-check-result".to_owned()),
        );
        action.payload.insert(
            ICINGA2_ACTION_PAYLOAD_KEY.to_owned(),
            json!(HashMap::from([
                ("filter".to_owned(), Value::String("filter_value".to_owned())),
                ("type".to_owned(), Value::String("Host".to_owned()))
            ])),
        );

        // Act
        let result = executor.parse_action(&action);

        // Assert
        let mut expected_payload = Map::new();
        expected_payload.insert("filter".to_owned(), Value::String("filter_value".to_owned()));
        expected_payload.insert("type".to_owned(), Value::String("Host".to_owned()));

        assert_eq!(
            Ok(Icinga2Action { name: "process-check-result", payload: Some(&expected_payload) }),
            result
        );
    }
}
