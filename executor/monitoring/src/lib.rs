use log::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tornado_common_api::RetriableError;
use tornado_common_api::{Action, Payload};
use tornado_executor_common::{ExecutorError, StatelessExecutor};
use tornado_executor_director::config::DirectorClientConfig;
use tornado_executor_director::{
    DirectorAction, DirectorActionName, DirectorExecutor,
    ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE,
};
use tornado_executor_icinga2::config::Icinga2ClientConfig;
use tornado_executor_icinga2::{
    Icinga2Action, Icinga2Executor, ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE,
};

pub const MONITORING_ACTION_NAME_KEY: &str = "action_name";
pub const ICINGA_FIELD_FOR_SPECIFYING_HOST: &str = "host";
pub const ICINGA_FIELD_FOR_SPECIFYING_SERVICE: &str = "service";
const PROCESS_CHECK_RESULT_SUBURL: &str = "process-check-result";

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "action_name")]
pub enum MonitoringAction {
    #[serde(rename = "create_and_or_process_host_passive_check_result")]
    Host { process_check_result_payload: Payload, host_creation_payload: Payload },
    #[serde(rename = "create_and_or_process_service_passive_check_result")]
    Service {
        process_check_result_payload: Payload,
        host_creation_payload: Payload,
        service_creation_payload: Payload,
    },
}

impl MonitoringAction {
    // Transforms the MonitoringAction into the actions needed to call the IcingaExecutor and the
    // DirectorExecutor.
    // Returns a triple, with these elements:
    // 1. Icinga2Action that will perform the process-check-result through the IcingaExecutor
    // 2. DirectorAction that will perform the creation of the host through the DirectorAction
    // 3. Option<DirectorAction> that will perform the creation of the service through the
    // DirectorAction. This is Some if MonitoringAction is of type Service, None otherwise
    pub fn to_sub_actions(
        &self,
    ) -> Result<(Icinga2Action, DirectorAction, Option<DirectorAction>), ExecutorError> {
        match &self {
            MonitoringAction::Host { process_check_result_payload, host_creation_payload } => {
                if process_check_result_payload.get(ICINGA_FIELD_FOR_SPECIFYING_HOST).is_none() {
                    return Err(ExecutorError::ConfigurationError { message: format!("Monitoring action expects that Icinga objects affected by the action are specified with field '{}' inside '{}' for action '{}'", ICINGA_FIELD_FOR_SPECIFYING_HOST, "process_check_result_payload", "create_and_or_process_host_passive_check_result" ) });
                };
                Ok((
                    Icinga2Action {
                        name: PROCESS_CHECK_RESULT_SUBURL,
                        payload: Some(process_check_result_payload),
                    },
                    DirectorAction {
                        name: DirectorActionName::CreateHost,
                        payload: host_creation_payload,
                        live_creation: true,
                    },
                    None,
                ))
            }
            MonitoringAction::Service {
                process_check_result_payload,
                host_creation_payload,
                service_creation_payload,
            } => {
                if process_check_result_payload.get(ICINGA_FIELD_FOR_SPECIFYING_SERVICE).is_none() {
                    return Err(ExecutorError::ConfigurationError { message: format!("Monitoring action expects that Icinga objects affected by the action are specified with field '{}' inside '{}' for action '{}'", ICINGA_FIELD_FOR_SPECIFYING_SERVICE, "process_check_result_payload", "create_and_or_process_service_passive_check_result" ) });
                };
                Ok((
                    Icinga2Action {
                        name: PROCESS_CHECK_RESULT_SUBURL,
                        payload: Some(process_check_result_payload),
                    },
                    DirectorAction {
                        name: DirectorActionName::CreateHost,
                        payload: host_creation_payload,
                        live_creation: true,
                    },
                    Some(DirectorAction {
                        name: DirectorActionName::CreateService,
                        payload: service_creation_payload,
                        live_creation: true,
                    }),
                ))
            }
        }
    }
}

/// An executor that performs a process check result and, if needed, creates the underneath host/service
#[derive(Clone)]
pub struct MonitoringExecutor {
    icinga_executor: Icinga2Executor,
    director_executor: DirectorExecutor,
}

impl std::fmt::Display for MonitoringExecutor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("MonitoringExecutor")?;
        Ok(())
    }
}

impl MonitoringExecutor {
    pub fn new(
        icinga2_client_config: Icinga2ClientConfig,
        director_client_config: DirectorClientConfig,
    ) -> Result<MonitoringExecutor, ExecutorError> {
        Ok(MonitoringExecutor {
            icinga_executor: Icinga2Executor::new(icinga2_client_config)?,
            director_executor: DirectorExecutor::new(director_client_config)?,
        })
    }

    pub fn parse_monitoring_action(payload: &Payload) -> Result<MonitoringAction, ExecutorError> {
        serde_json::to_value(payload).and_then(serde_json::from_value).map_err(|err| {
            ExecutorError::ConfigurationError {
                message: format!("Invalid Monitoring Action configuration. Err: {:?}", err),
            }
        })
    }

    async fn perform_creation_of_icinga_objects<'a>(
        &self,
        director_host_creation_action: DirectorAction<'a>,
        director_service_creation_action: Option<DirectorAction<'a>>,
    ) -> Result<(), ExecutorError> {
        let host_creation_result =
            self.director_executor.perform_request(director_host_creation_action).await;
        match host_creation_result {
            Ok(()) => {
                debug!("MonitoringExecutor - Director host creation action successfully performed");
                Ok(())
            }
            Err(ExecutorError::ActionExecutionError { message, code: Some(code), .. })
                if code.eq(ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE) =>
            {
                debug!("MonitoringExecutor - Director host creation action failed with message {:?}. Looks like the host already exists in Icinga.", message);
                Ok(())
            }
            Err(err) => {
                error!(
                    "MonitoringExecutor - Director host creation action failed with error {:?}.",
                    err
                );
                Err(ExecutorError::ActionExecutionError {
                    message: format!("MonitoringExecutor - Error during the host creation. DirectorExecutor failed with error: {:?}", err), can_retry: err.can_retry(), code: None, data: Default::default(), })
            }
        }?;

        if let Some(director_service_creation_action) = director_service_creation_action {
            let service_creation_result =
                self.director_executor.perform_request(director_service_creation_action).await;
            match service_creation_result {
                Ok(()) => {
                    debug!("MonitoringExecutor - Director service creation action successfully performed");
                    Ok(())
                }
                Err(ExecutorError::ActionExecutionError { message, code: Some(code), .. })
                    if code.eq(ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE) =>
                {
                    debug!("MonitoringExecutor - Director service creation action failed with message {:?}. Looks like the host already exists in Icinga.", message);
                    Ok(())
                }
                Err(err) => {
                    error!("MonitoringExecutor - Director service creation action failed with error {:?}.", err);
                    Err(ExecutorError::ActionExecutionError { message: format!("MonitoringExecutor - Error during the service creation. DirectorExecutor failed with error: {:?}", err), can_retry: err.can_retry(), code: None, data: Default::default(), })
                }
            }?;
        };
        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl StatelessExecutor for MonitoringExecutor {
    #[tracing::instrument(level = "info", skip_all, err, fields(otel.name = format!("Execute Action: {}", &action.id).as_str(), otel.kind = "Consumer"))]
    async fn execute(&self, action: Arc<Action>) -> Result<(), ExecutorError> {
        trace!("MonitoringExecutor - received action: \n[{:?}]", action);

        let monitoring_action = MonitoringExecutor::parse_monitoring_action(&action.payload)?;

        let (icinga2_action, director_host_creation_action, director_service_creation_action) =
            monitoring_action.to_sub_actions()?;
        let icinga2_action_result = self.icinga_executor.perform_request(&icinga2_action).await;

        match icinga2_action_result {
            Ok(_) => {
                debug!("MonitoringExecutor - Process check result correctly performed");
                Ok(())
            }
            Err(ExecutorError::ActionExecutionError { message, code: Some(code), .. })
                if code.eq(ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE) =>
            {
                debug!("MonitoringExecutor - Process check result action failed with message {:?}. Looks like Icinga2 object does not exist yet. Proceeding with the creation of the object..", message);
                self.perform_creation_of_icinga_objects(
                    director_host_creation_action,
                    director_service_creation_action,
                )
                .await?;
                self.icinga_executor.perform_request(&icinga2_action).await.map_err(|err| ExecutorError::ActionExecutionError { message: format!("MonitoringExecutor - Error while performing the process check result after the object creation. IcingaExecutor failed with error: {:?}", err), can_retry: err.can_retry(), code: None, data: Default::default(), })
            }
            Err(err) => {
                error!(
                    "MonitoringExecutor - Process check result action failed with error {:?}.",
                    err
                );
                Err(ExecutorError::ActionExecutionError { message: format!("MonitoringExecutor - Error while performing the process check result. IcingaExecutor failed with error: {:?}", err), can_retry: err.can_retry(), code: None, data: Default::default(), })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use maplit::*;
    use serde_json::json;
    use tornado_common_api::{Action, Map, Value};

    #[tokio::test]
    async fn should_fail_if_action_missing() {
        // Arrange
        let executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let action = Action::new("");

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("Invalid Monitoring Action configuration"))
            }
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn should_throw_error_if_action_name_is_not_valid() {
        // Arrange
        let executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action
            .payload
            .insert("action_name".to_owned(), Value::String("my_invalid_action".to_owned()));
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            json!(hashmap!(
                "host".to_owned() => Value::String("myhost".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Object(Map::new()));
        action.payload.insert("service_creation_payload".to_owned(), Value::Object(Map::new()));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("Invalid Monitoring Action configuration"))
            }
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn should_throw_error_if_service_action_but_service_creation_payload_not_given() {
        // Arrange
        let executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::String("create_and_or_process_service_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            json!(hashmap!(
                "service".to_owned() => Value::String("myservice".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Object(Map::new()));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("Invalid Monitoring Action configuration"))
            }
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn should_return_ok_if_action_name_is_valid() {
        // Arrange
        let mock_server = MockServer::start();

        mock_server.mock(|when, then| {
            when.method(POST).path("/v1/actions/process-check-result");
            then.status(200).body("{\"results\":[{\"code\":200.0,\"status\":\"Successfully processed check result for object 'myhost'.\"}]}");
        });

        let executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: mock_server.url(""),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::String("create_and_or_process_host_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            json!(hashmap!(
                "host".to_owned() => Value::String("myhost".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Object(Map::new()));
        action.payload.insert("service_creation_payload".to_owned(), Value::Object(Map::new()));

        // Act
        let result = executor.execute(action.into()).await;

        println!("{:?}", result);

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn should_throw_error_if_process_check_result_host_not_specified_with_host_field() {
        // Arrange
        let executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::String("create_and_or_process_host_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            json!(hashmap!(
                "filter".to_owned() => Value::String("host.name==\"myhost\"".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Object(Map::new()));

        // Act
        let result = executor.execute(action.into()).await;

        println!("{:?}", result);

        // Assert
        assert!(result.is_err());
        assert_eq!(result, Err(ExecutorError::ConfigurationError { message: "Monitoring action expects that Icinga objects affected by the action are specified with field 'host' inside 'process_check_result_payload' for action 'create_and_or_process_host_passive_check_result'".to_string() }))
    }

    #[tokio::test]
    async fn should_throw_error_if_process_check_result_service_not_specified_with_service_field() {
        // Arrange
        let executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::String("create_and_or_process_service_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            json!(hashmap!(
                "filter".to_owned() => Value::String("host.name==\"myhost\"".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Object(Map::new()));
        action.payload.insert("service_creation_payload".to_owned(), Value::Object(Map::new()));

        // Act
        let result = executor.execute(action.into()).await;

        println!("{:?}", result);

        // Assert
        assert!(result.is_err());
        assert_eq!(result, Err(ExecutorError::ConfigurationError { message: "Monitoring action expects that Icinga objects affected by the action are specified with field 'service' inside 'process_check_result_payload' for action 'create_and_or_process_service_passive_check_result'".to_string() }))
    }

    #[tokio::test]
    async fn should_return_ok_if_action_type_is_host_and_service_creation_payload_not_given() {
        // Arrange
        let mock_server = MockServer::start();

        mock_server.mock(|when, then| {
            when.method(POST).path("/v1/actions/process-check-result");
            then.status(200).body("{\"results\":[{\"code\":200.0,\"status\":\"Successfully processed check result for object 'myhost'.\"}]}");
        });

        let executor = MonitoringExecutor::new(
            Icinga2ClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: mock_server.url(""),
            },
            DirectorClientConfig {
                timeout_secs: None,
                username: "".to_owned(),
                password: "".to_owned(),
                disable_ssl_verification: true,
                server_api_url: "".to_owned(),
            },
        )
        .unwrap();

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::String("create_and_or_process_host_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            json!(hashmap!(
                "host".to_owned() => Value::String("myhost".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Object(Map::new()));

        // Act
        let result = executor.execute(action.into()).await;

        println!("{:?}", result);

        // Assert
        assert!(result.is_ok());
    }
}
