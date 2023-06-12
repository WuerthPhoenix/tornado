use action::SimpleCreateAndProcess;
use log::*;
use std::{future::Future, pin::Pin, sync::Arc};
use tornado_common_api::RetriableError;
use tornado_common_api::{Action, Payload};
use tornado_executor_common::{ExecutorError, StatelessExecutor};
use tornado_executor_director::config::DirectorClientConfig;
use tornado_executor_director::{
    DirectorAction, DirectorExecutor, ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE,
};
use tornado_executor_icinga2::config::Icinga2ClientConfig;
use tornado_executor_icinga2::{
    Icinga2Action, Icinga2Executor, ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE,
};
use tracing::instrument;

pub const MONITORING_ACTION_NAME_KEY: &str = "action_name";

mod action;
pub mod migration;

/// An executor that performs a process check result and, if needed, creates the underneath host/service
#[derive(Clone)]
pub struct SmartMonitoringExecutor {
    icinga_executor: Arc<Icinga2Executor>,
    director_executor: DirectorExecutor,
}

impl std::fmt::Display for SmartMonitoringExecutor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("SmartMonitoringExecutor")?;
        Ok(())
    }
}

impl SmartMonitoringExecutor {
    pub fn new(
        icinga2_client_config: Icinga2ClientConfig,
        director_client_config: DirectorClientConfig,
    ) -> Result<SmartMonitoringExecutor, ExecutorError> {
        Ok(SmartMonitoringExecutor {
            icinga_executor: Arc::new(Icinga2Executor::new(icinga2_client_config)?),
            director_executor: DirectorExecutor::new(director_client_config)?,
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
                debug!("SmartMonitoringExecutor - Director host creation action successfully performed");
                Ok(())
            }
            Err(ExecutorError::ActionExecutionError { message, code: Some(code), .. })
                if code.eq(ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE) =>
            {
                debug!("SmartMonitoringExecutor - Director host creation action failed with message {:?}. Looks like the host already exists in Icinga.", message);
                Ok(())
            }
            Err(err) => {
                error!(
                    "SmartMonitoringExecutor - Director host creation action failed with error {:?}.",
                    err
                );
                Err(ExecutorError::ActionExecutionError { message: format!("SmartMonitoringExecutor - Error during the host creation. DirectorExecutor failed with error: {:?}", err), can_retry: err.can_retry(), code: None, data: Default::default(), })
            }
        }?;

        if let Some(director_service_creation_action) = director_service_creation_action {
            let service_creation_result =
                self.director_executor.perform_request(director_service_creation_action).await;
            match service_creation_result {
                Ok(()) => {
                    debug!("SmartMonitoringExecutor - Director service creation action successfully performed");
                    Ok(())
                }
                Err(ExecutorError::ActionExecutionError { message, code: Some(code), .. })
                    if code.eq(ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE) =>
                {
                    debug!("SmartMonitoringExecutor - Director service creation action failed with message {:?}. Looks like the host already exists in Icinga.", message);
                    Ok(())
                }
                Err(err) => {
                    error!("SmartMonitoringExecutor - Director service creation action failed with error {:?}.", err);
                    Err(ExecutorError::ActionExecutionError { message: format!("SmartMonitoringExecutor - Error during the service creation. DirectorExecutor failed with error: {:?}", err), can_retry: err.can_retry(), code: None, data: Default::default(), })
                }
            }?;
        };
        Ok(())
    }

    fn set_state(
        icinga_executor: Arc<Icinga2Executor>,
        icinga2_action: Icinga2ActionOwned,
        host_name: Option<String>,
        service_name: Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ExecutorError>>>> {
        Box::pin(async move {
            match icinga_executor.perform_request(&(&icinga2_action).into()).await {
                Ok(()) => {
                    trace!("SmartMonitoringExecutor - process_check_result for object host [{:?}] service [{:?}] successfully performed.", host_name, service_name);
                    Ok(())
                }
                Err(err) => {
                    warn!("SmartMonitoringExecutor - process_check_result for object host [{:?}] service [{:?}] completed with errors. err: {:?}", host_name, service_name, err);
                    Err(err)
                }
            }
        })
    }

    #[instrument(level = "debug", name = "SmartMonitoring", err, skip_all, fields(otel.name = format!("Perform SmartMonitoring Action for host: [{:?}], service: [{:?}]", &host_name, &service_name).as_str()))]
    async fn execute_smart_monitoring_action(
        &self,
        icinga2_action: &Icinga2Action<'_>,
        director_host_creation_action: DirectorAction<'_>,
        director_service_creation_action: Option<DirectorAction<'_>>,
        host_name: Option<String>,
        service_name: Option<String>,
    ) -> Result<(), ExecutorError> {
        let icinga2_action_result = self.icinga_executor.perform_request(icinga2_action).await;

        match icinga2_action_result {
            Ok(_) => {
                debug!("SmartMonitoringExecutor - Process check result correctly performed");
                Ok(())
            }
            Err(ExecutorError::ActionExecutionError { message, code: Some(code), .. })
                if code.eq(ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE) =>
            {
                debug!("SmartMonitoringExecutor - Process check result action failed with message {:?}. Looks like Icinga2 object does not exist yet. Proceeding with the creation of the object..", message);
                self.perform_creation_of_icinga_objects(
                    director_host_creation_action,
                    director_service_creation_action,
                )
                .await?;

                SmartMonitoringExecutor::set_state(
                    self.icinga_executor.clone(),
                    Icinga2ActionOwned {
                        name: icinga2_action.name.to_owned(),
                        payload: icinga2_action.payload.cloned(),
                    },
                    host_name,
                    service_name,
                )
                .await
            }
            Err(err) => {
                error!(
                    "SmartMonitoringExecutor - Process check result action failed with error {:?}.",
                    err
                );
                Err(ExecutorError::ActionExecutionError { message: format!("SmartMonitoringExecutor - Error while performing the process check result. IcingaExecutor failed with error: {:?}", err), can_retry: err.can_retry(), code: None, data: Default::default() })
            }
        }
    }
}

#[async_trait::async_trait(?Send)]
impl StatelessExecutor for SmartMonitoringExecutor {
    #[tracing::instrument(level = "info", skip_all, err, fields(otel.name = format!("Execute Action: {}", & action.id).as_str(), otel.kind = "Consumer"))]
    async fn execute(&self, action: Arc<Action>) -> Result<(), ExecutorError> {
        trace!("SmartMonitoringExecutor - received action: \n[{:?}]", action);

        let extraction_params_guard =
            tracing::debug_span!("Extract parameters for Executor").entered();
        let mut monitoring_action = SimpleCreateAndProcess::new(&action)?;
        let host_name = monitoring_action.get_host_name().map(|val| val.to_owned());
        let service_name = monitoring_action.get_service_name().map(|val| val.to_owned());

        let (icinga2_action, director_host_creation_action, director_service_creation_action) =
            monitoring_action.build_sub_actions()?;
        extraction_params_guard.exit();

        self.execute_smart_monitoring_action(
            &icinga2_action,
            director_host_creation_action,
            director_service_creation_action,
            host_name,
            service_name,
        )
        .await
    }
}

pub struct Icinga2ActionOwned {
    pub name: String,
    pub payload: Option<Payload>,
}

impl<'a> From<&'a Icinga2ActionOwned> for Icinga2Action<'a> {
    fn from(action_owned: &'a Icinga2ActionOwned) -> Self {
        Icinga2Action { name: &action_owned.name, payload: action_owned.payload.as_ref() }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use maplit::hashmap;
    use serde_json::json;
    use tornado_common_api::{Action, Map, Value};

    #[tokio::test]
    async fn should_fail_if_action_data_is_missing() {
        // Arrange
        let executor = SmartMonitoringExecutor::new(
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
            Err(ExecutorError::ConfigurationError { .. }) => {}
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn should_return_ok_if_action_is_valid() {
        // Arrange
        let mock_server = MockServer::start();

        mock_server.mock(|when, then| {
            when.method(POST).path("/v1/actions/process-check-result");
            then.status(200).body("{\"results\":[{\"code\":200.0,\"status\":\"Successfully processed check result for object 'myhost'.\"}]}");
        });

        let executor = SmartMonitoringExecutor::new(
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
        action.payload.insert("check_result".to_owned(), Value::Object(Map::new()));
        action.payload.insert(
            "host".to_owned(),
            json!(hashmap!(
                "object_name".to_owned() => Value::String("myhost".to_owned()),
            )),
        );
        action.payload.insert(
            "service".to_owned(),
            json!(hashmap!(
                "object_name".to_owned() => Value::String("myservice".to_owned()),
            )),
        );

        // Act
        let result = executor.execute(action.into()).await;

        println!("{:?}", result);

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn should_throw_error_if_process_check_result_missing() {
        // Arrange
        let executor = SmartMonitoringExecutor::new(
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
            "host".to_owned(),
            json!(hashmap!(
                "object_name".to_owned() => Value::String("myhost".to_owned()),
            )),
        );
        action.payload.insert(
            "service".to_owned(),
            json!(hashmap!(
                "object_name".to_owned() => Value::String("myservice".to_owned()),
            )),
        );

        // Act
        let result = executor.execute(action.into()).await;

        println!("{:?}", result);

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("check_result"))
            }
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn should_throw_error_if_host_name_missing() {
        // Arrange
        let executor = SmartMonitoringExecutor::new(
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
        action.payload.insert("check_result".to_owned(), Value::Object(Map::new()));
        action.payload.insert("host".to_owned(), Value::Object(Map::new()));
        action.payload.insert(
            "service".to_owned(),
            json!(hashmap!(
                "object_name".to_owned() => Value::String("myservice".to_owned()),
            )),
        );

        // Act
        let result = executor.execute(action.into()).await;

        println!("{:?}", result);

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("host"));
                assert!(message.contains("object_name"));
            }
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn should_throw_error_if_service_name_missing() {
        // Arrange
        let executor = SmartMonitoringExecutor::new(
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
        action.payload.insert("check_result".to_owned(), Value::Object(Map::new()));
        action.payload.insert(
            "host".to_owned(),
            json!(hashmap!(
                "object_name".to_owned() => Value::String("myhost".to_owned()),
            )),
        );
        action.payload.insert("service".to_owned(), Value::Object(Map::new()));

        // Act
        let result = executor.execute(action.into()).await;

        println!("{:?}", result);

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("service"));
                assert!(message.contains("object_name"));
            }
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn should_return_ok_if_action_type_is_host_and_service_creation_payload_not_given() {
        // Arrange
        let mock_server = MockServer::start();

        mock_server.mock(|when, then| {
            when.method(POST).path("/v1/actions/process-check-result");
            then.status(200).body("{\"results\":[{\"code\":200.0,\"status\":\"Successfully processed check result for object 'myhost'.\"}]}");
        });

        let executor = SmartMonitoringExecutor::new(
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
        action.payload.insert("check_result".to_owned(), Value::Object(Map::new()));
        action.payload.insert(
            "host".to_owned(),
            json!(hashmap!(
                "object_name".to_owned() => Value::String("myhost".to_owned()),
            )),
        );

        // Act
        let result = executor.execute(action.into()).await;

        println!("{:?}", result);

        // Assert
        assert!(result.is_ok());
    }
}
