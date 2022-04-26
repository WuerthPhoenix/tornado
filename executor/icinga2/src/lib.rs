use crate::client::ApiClient;
use crate::config::Icinga2ClientConfig;
use log::*;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tornado_common_api::Payload;
use tornado_common_api::{Action, ValueExt};
use tornado_executor_common::{ExecutorError, StatelessExecutor};
use tracing::instrument;

pub mod client;
pub mod config;

pub const ICINGA2_ACTION_NAME_KEY: &str = "icinga2_action_name";
pub const ICINGA2_ACTION_PAYLOAD_KEY: &str = "icinga2_action_payload";

const ICINGA2_OBJECT_NOT_EXISTING_RESPONSE: &str = "No objects found";
const ICINGA2_OBJECT_NOT_EXISTING_STATUS_CODE: u16 = 404;
pub const ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE: &str = "IcingaObjectNotExisting";
const ICINGA2_PROCESS_CHECK_RESULT_WAS_DISCARDED_RESULT_CODE: u16 = 409;
const ICINGA2_PROCESS_CHECK_RESULT_WAS_DISCARDED_RESULT_STATUS: &str =
    "Newer check result already present";

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

    #[instrument(level = "debug", name = "Extract parameters for Executor", skip_all)]
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

    #[instrument(level = "debug", name = "IcingaRequest", err, skip_all, fields(otel.name = format!("Send request of type: [{}] to Icinga2 ", &icinga2_action.name).as_str()))]
    pub async fn perform_request<'a>(
        &self,
        icinga2_action: &'a Icinga2Action<'a>,
    ) -> Result<(), ExecutorError> {
        let payload = &icinga2_action.payload;
        let response = self.api_client.api_post_action(icinga2_action.name, payload).await?;

        let method = response.method;
        let url = response.url;

        let response_status = response.response.status();

        let icinga2_action_response: Icinga2ActionResponse =
            response.response.json().await.map_err(|err| {
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

        match Icinga2ActionResponseType::new(icinga2_action_response, &response_status) {
            Icinga2ActionResponseType::ObjectNotFoundError(response) => {
                Self::handle_object_not_found_error(
                    payload,
                    method,
                    &url,
                    response_status,
                    &response,
                )
            }
            Icinga2ActionResponseType::UnrecoverableError(response) => {
                Self::handle_unrecoverable_error(payload, method, &url, response_status, &response)
            }
            Icinga2ActionResponseType::GenericRecoverableError(response) => {
                Self::handle_generic_recoverable_error(
                    payload,
                    method,
                    &url,
                    response_status,
                    &response,
                )
            }
            Icinga2ActionResponseType::Ok(_response) => Self::handle_ok(),
        }
    }

    fn handle_ok() -> Result<(), ExecutorError> {
        debug!("Icinga2Executor - Data correctly sent to Icinga2 API");
        Ok(())
    }

    fn handle_generic_recoverable_error(
        payload: &Option<&Payload>,
        method: &str,
        url: &String,
        response_status: StatusCode,
        response: &Icinga2ActionResponse,
    ) -> Result<(), ExecutorError> {
        Err(ExecutorError::ActionExecutionError {
            can_retry: true,
            message: format!(
                "Icinga2Executor - Icinga2 API returned a recoverable error. Response status: {}. Response body: {}", response_status, serde_json::to_string(&response)?
            ),
            code: None,
            data: to_err_data(method, &url, payload)?.into()
        })
    }

    fn handle_unrecoverable_error(
        payload: &Option<&Payload>,
        method: &str,
        url: &String,
        response_status: StatusCode,
        response: &Icinga2ActionResponse,
    ) -> Result<(), ExecutorError> {
        response.log_unrecoverable_errors()?;
        Err(ExecutorError::ActionExecutionError {
            can_retry: false,
            message: format!(
                "Icinga2Executor - Icinga2 API returned an unrecoverable error. Response status: {}. Response body: {}", response_status, serde_json::to_string(&response)?
            ),
            code: None,
            data: to_err_data(method, &url, payload)?.into()
        })
    }

    fn handle_object_not_found_error(
        payload: &Option<&Payload>,
        method: &str,
        url: &String,
        response_status: StatusCode,
        response: &Icinga2ActionResponse,
    ) -> Result<(), ExecutorError> {
        Err(ExecutorError::ActionExecutionError {
            message: format!("Icinga2Executor - Icinga2 API returned an error, object seems to be not existing in Icinga2. Response status: {}. Response body: {}", response_status, serde_json::to_string(&response)?),
            can_retry: true,
            code: Some(ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE),
            data: to_err_data(method, &url, payload)?.into()
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Icinga2ActionResponse {
    ErrorResponse(ErrorBody),
    OkResponse(ResultsBody),
}

pub enum Icinga2ActionResponseType {
    ObjectNotFoundError(Icinga2ActionResponse),
    UnrecoverableError(Icinga2ActionResponse),
    GenericRecoverableError(Icinga2ActionResponse),
    Ok(Icinga2ActionResponse),
}

impl Icinga2ActionResponseType {
    pub fn new(
        icinga2_action_response: Icinga2ActionResponse,
        response_status_code: &StatusCode,
    ) -> Self {
        if response_status_code.eq(&ICINGA2_OBJECT_NOT_EXISTING_STATUS_CODE)
            && icinga2_action_response.is_no_object_found_error()
        {
            Self::ObjectNotFoundError(icinga2_action_response)
        } else if !response_status_code.is_success() && !icinga2_action_response.is_recoverable() {
            Self::UnrecoverableError(icinga2_action_response)
        } else if !response_status_code.is_success() {
            Self::GenericRecoverableError(icinga2_action_response)
        } else {
            Self::Ok(icinga2_action_response)
        }
    }
}

impl Icinga2ActionResponse {
    fn is_no_object_found_error(&self) -> bool {
        match &self {
            Icinga2ActionResponse::ErrorResponse(error_body) => {
                error_body.status.contains(ICINGA2_OBJECT_NOT_EXISTING_RESPONSE)
            }
            Icinga2ActionResponse::OkResponse(_) => false,
        }
    }

    fn is_recoverable(&self) -> bool {
        match &self {
            // We consider any "global" error returned by Icinga 2 as retryable
            // to be on the safe side (trying our best to execute actions)
            Icinga2ActionResponse::ErrorResponse(_body) => true,
            Icinga2ActionResponse::OkResponse(body) => body.is_recoverable(),
        }
    }

    fn log_unrecoverable_errors(&self) -> Result<(), ExecutorError> {
        match &self {
            // Currently no "global" error is considered as unrecoverable
            Icinga2ActionResponse::ErrorResponse(_body) => Ok(()),
            Icinga2ActionResponse::OkResponse(body) => body.log_unrecoverable_errors(),
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: f64,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct ResultsBody {
    pub results: Vec<Icinga2Result>,
}

impl ResultsBody {
    fn is_recoverable(&self) -> bool {
        let mut error_results =
            self.results.iter().filter(|result| !result.is_successful()).peekable();
        let all_results_are_success = error_results.peek().is_none();
        let any_error_is_recoverable = error_results.all(|result| result.is_recoverable());
        all_results_are_success || any_error_is_recoverable
    }

    fn log_unrecoverable_errors(&self) -> Result<(), ExecutorError> {
        for result in &self.results {
            if !result.is_recoverable() {
                error!("Unrecoverable error encountered: {}", result.to_log_message()?);
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct Icinga2Result {
    pub code: f64,
    pub status: String,
    #[serde(flatten)]
    additional_fields: Map<String, Value>,
}

impl Icinga2Result {
    pub fn is_successful(&self) -> bool {
        (self.code as u16) < 300 && (self.code as u16) >= 200
    }
    pub fn is_recoverable(&self) -> bool {
        !self.is_discarded_process_check_result()
    }

    pub fn is_discarded_process_check_result(&self) -> bool {
        (self.code as u16) == ICINGA2_PROCESS_CHECK_RESULT_WAS_DISCARDED_RESULT_CODE
            && self.status.contains(ICINGA2_PROCESS_CHECK_RESULT_WAS_DISCARDED_RESULT_STATUS)
    }

    pub fn to_log_message(&self) -> Result<String, ExecutorError> {
        let tag = if self.is_discarded_process_check_result() {
            "DISCARDED_PROCESS_CHECK_RESULT".to_string()
        } else {
            "".to_string()
        };
        let result_value = serde_json::to_value(self)?;
        let mut message_object = Value::Object(Map::new());
        message_object
            .get_map_mut()
            .map(|map| {
                map.insert("tag".to_string(), serde_json::Value::String(tag));
                map.insert("content".to_string(), result_value);
            })
            .ok_or(ExecutorError::JsonError { cause: "".to_string() })?;
        serde_json::to_string(&message_object).map_err(|err| err.into())
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
    #[tracing::instrument(level = "info", skip_all, err, fields(otel.name = format!("Execute Action: {}", &action.id).as_str(), otel.kind = "Consumer"))]
    async fn execute(&self, action: Arc<Action>) -> Result<(), ExecutorError> {
        trace!("Icinga2Executor - received action: \n[{:?}]", action);
        let action = self.parse_action(&action)?;

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

    #[test]
    fn should_deserialize_icinga2_result() {
        // Arrange
        let result = r#"{
            "code": 200.0,
            "legacy_id": 26.0,
            "name": "icinga2-satellite1.localdomain!ping4!7e7861c8-8008-4e8d-9910-2a0bb26921bd",
            "status": "Successfully added comment 'icinga2-satellite1.localdomain!ping4!7e7861c8-8008-4e8d-9910-2a0bb26921bd' for object 'icinga2-satellite1.localdomain!ping4'."
        }"#;

        // Act
        let icinga2_result: Icinga2Result = serde_json::from_str(result).unwrap();

        // Assert
        assert_eq!(icinga2_result.code as u16, 200);
        assert_eq!(icinga2_result.status, "Successfully added comment 'icinga2-satellite1.localdomain!ping4!7e7861c8-8008-4e8d-9910-2a0bb26921bd' for object 'icinga2-satellite1.localdomain!ping4'.");
        assert!(icinga2_result.additional_fields.get("code").is_none());
        assert!(icinga2_result.additional_fields.get("status").is_none());
        assert!(icinga2_result.additional_fields.get("name").is_some());
        assert!(icinga2_result.additional_fields.get("legacy_id").is_some());
    }

    #[test]
    fn should_return_if_icinga2_result_is_successful() {
        // Arrange
        let successful_icinga2_results = vec![
            Icinga2Result {
                code: 200.0,
                status: "".to_string(),
                additional_fields: Default::default(),
            },
            Icinga2Result {
                code: 201.0,
                status: "".to_string(),
                additional_fields: Default::default(),
            },
            Icinga2Result {
                code: 299.0,
                status: "".to_string(),
                additional_fields: Default::default(),
            },
        ];
        let unsuccessful_icinga2_results = vec![
            Icinga2Result {
                code: 150.0,
                status: "".to_string(),
                additional_fields: Default::default(),
            },
            Icinga2Result {
                code: 300.0,
                status: "".to_string(),
                additional_fields: Default::default(),
            },
            Icinga2Result {
                code: 409.0,
                status: "".to_string(),
                additional_fields: Default::default(),
            },
            Icinga2Result {
                code: 500.0,
                status: "".to_string(),
                additional_fields: Default::default(),
            },
        ];

        // Assert
        assert!(successful_icinga2_results.iter().all(|result| result.is_successful()));
        assert!(unsuccessful_icinga2_results.iter().all(|result| !result.is_successful()));
    }

    #[test]
    fn icinga2_result_is_discarded_process_check_result_should_return_true() {
        // Arrange
        let result = Icinga2Result {
            code: 409.0,
            status: "Newer check result already present. Check result for 'myhost' was discarded."
                .to_string(),
            additional_fields: Default::default(),
        };

        // Assert
        assert!(result.is_discarded_process_check_result());
        assert!(!result.is_recoverable());
    }

    #[test]
    fn icinga2_result_is_discarded_process_check_result_should_return_false() {
        // Arrange
        let result_1 = Icinga2Result {
            code: 500.0,
            status: "Newer check result already present. Check result for 'myhost' was discarded."
                .to_string(),
            additional_fields: Default::default(),
        };

        let result_2 = Icinga2Result {
            code: 409.0,
            status: "Conflict".to_string(),
            additional_fields: Default::default(),
        };

        // Assert
        assert!(!result_1.is_discarded_process_check_result());
        assert!(result_1.is_recoverable());
        assert!(!result_2.is_discarded_process_check_result());
        assert!(result_2.is_recoverable());
    }

    #[test]
    fn icinga2_result_should_return_log_message() {
        // Arrange
        let result = Icinga2Result {
            code: 409.0,
            status: "Newer check result already present. Check result for 'myhost' was discarded."
                .to_string(),
            additional_fields: Default::default(),
        };

        // Act
        let message = result.to_log_message().unwrap();

        // Assert
        let expected = r#"{"content":{"code":409.0,"status":"Newer check result already present. Check result for 'myhost' was discarded."},"tag":"DISCARDED_PROCESS_CHECK_RESULT"}"#;
        assert_eq!(message, expected);
    }

    #[test]
    fn icinga2_result_should_return_log_message_with_empty_tag_for_unknown_error() {
        // Arrange
        let result = Icinga2Result {
            code: 500.0,
            status: "Internal server error.".to_string(),
            additional_fields: Default::default(),
        };

        // Act
        let message = result.to_log_message().unwrap();

        // Assert
        let expected = r#"{"content":{"code":500.0,"status":"Internal server error."},"tag":""}"#;
        assert_eq!(message, expected);
    }

    #[test]
    fn should_deserialize_results_body() {
        // Arrange
        let result = r#"{
    "results": [
        {
            "code": 200.0,
            "legacy_id": 26.0,
            "name": "icinga2-satellite1.localdomain!ping4!7e7861c8-8008-4e8d-9910-2a0bb26921bd",
            "status": "Successfully added comment 'icinga2-satellite1.localdomain!ping4!7e7861c8-8008-4e8d-9910-2a0bb26921bd' for object 'icinga2-satellite1.localdomain!ping4'."
        },
        {
            "code": 500.0,
            "legacy_id": 27.0,
            "name": "icinga2-satellite2.localdomain!ping4!9a4c43f5-9407-a536-18bf-4a6cc4b73a9f",
            "status": "Successfully added comment 'icinga2-satellite2.localdomain!ping4!9a4c43f5-9407-a536-18bf-4a6cc4b73a9f' for object 'icinga2-satellite2.localdomain!ping4'."
        }
    ]
}"#;

        // Act
        let icinga2_response: Icinga2ActionResponse = serde_json::from_str(result).unwrap();

        // Assert
        match icinga2_response {
            Icinga2ActionResponse::OkResponse(results_body) => {
                assert_eq!(results_body.results.len(), 2);
                assert_eq!(results_body.results.get(0).unwrap().code as u16, 200);
                assert_eq!(results_body.results.get(1).unwrap().code as u16, 500);
            }
            Icinga2ActionResponse::ErrorResponse(_) => {
                assert!(false)
            }
        }
    }

    #[test]
    fn should_deserialize_icinga_error_response_body() {
        // Arrange
        let result = r#"{"error":404.0,"status":"No objects found."}"#;

        // Act
        let icinga2_response: Icinga2ActionResponse = serde_json::from_str(result).unwrap();

        // Assert
        match icinga2_response {
            Icinga2ActionResponse::ErrorResponse(error_body) => {
                assert_eq!(error_body.error as u16, 404);
                assert_eq!(error_body.status, "No objects found.");
            }
            Icinga2ActionResponse::OkResponse(_) => {
                assert!(false)
            }
        }
    }

    #[test]
    fn results_body_should_return_unrecoverable_if_all_results_are_recoverable_errors() {
        // Arrange
        let results_body = ResultsBody {
            results: vec![Icinga2Result {
                code: 409.0,
                status: "Newer check result already present. Check result for 'myhost' was discarded.".to_string(),
                additional_fields: Default::default(),
            },
          Icinga2Result {
              code: 409.0,
              status: "Newer check result already present. Check result for 'myhost' was discarded.".to_string(),
              additional_fields: Default::default(),
          }],
        };

        // Assert
        assert!(!results_body.is_recoverable());
    }

    #[test]
    fn results_body_should_return_unrecoverable_if_all_results_are_recoverable_errors_and_successes(
    ) {
        // Arrange
        let results_body = ResultsBody {
            results: vec![
                Icinga2Result {
                    code: 409.0,
                    status: "Newer check result already present. Check result for 'myhost' was discarded.".to_string(),
                    additional_fields: Default::default(),
                },
              Icinga2Result {
                  code: 200.0,
                  status: "Ok.".to_string(),
                  additional_fields: Default::default(),
              }
            ],
        };

        // Assert
        assert!(!results_body.is_recoverable());
    }

    #[test]
    fn results_body_should_return_recoverable_if_all_results_are_successes() {
        // Arrange
        let results_body = ResultsBody {
            results: vec![
                Icinga2Result {
                    code: 200.0,
                    status: "Ok.".to_string(),
                    additional_fields: Default::default(),
                },
                Icinga2Result {
                    code: 200.0,
                    status: "Ok.".to_string(),
                    additional_fields: Default::default(),
                },
            ],
        };

        // Assert
        assert!(results_body.is_recoverable());
    }

    #[test]
    fn results_body_should_return_recoverable_if_all_results_are_successes_or_recoverable_errors() {
        // Arrange
        let results_body = ResultsBody {
            results: vec![
                Icinga2Result {
                    code: 200.0,
                    status: "Ok.".to_string(),
                    additional_fields: Default::default(),
                },
                Icinga2Result {
                    code: 404.0,
                    status: "No objects found.".to_string(),
                    additional_fields: Default::default(),
                },
            ],
        };

        // Assert
        assert!(results_body.is_recoverable());
    }

    #[test]
    fn results_body_should_return_recoverable_if_results_is_empty() {
        // Arrange
        let results_body = ResultsBody { results: vec![] };

        // Assert
        assert!(results_body.is_recoverable());
    }

    #[test]
    fn is_no_object_found_error_should_return_true_for_such_response() {
        // Arrange
        let response = r#"{"error":404.0,"status":"No objects found."}"#;
        let response: Icinga2ActionResponse = serde_json::from_str(response).unwrap();

        // Assert
        assert!(response.is_no_object_found_error())
    }

    #[test]
    fn is_no_object_found_error_should_return_false_for_other_errors() {
        // Arrange
        let response = r#"{"error":400.0,"status":"Invalid request body: Error: [json.exception.parse_error.101] parse error at line 1, column 101: syntax error while parsing value - unexpected '}'; expected '[', '{', or a literal\n\n"}"#;
        let response: Icinga2ActionResponse = serde_json::from_str(response).unwrap();

        // Assert
        assert!(!response.is_no_object_found_error())
    }
}
