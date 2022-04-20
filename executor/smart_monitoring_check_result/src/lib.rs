use crate::config::SmartMonitoringCheckResultConfig;
use action::SimpleCreateAndProcess;
use log::*;
use maplit::hashmap;
use serde_json::Value;
use std::time::Duration;
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
pub mod config;
pub mod migration;

/// An executor that performs a process check result and, if needed, creates the underneath host/service
#[derive(Clone)]
pub struct SmartMonitoringExecutor {
    config: SmartMonitoringCheckResultConfig,
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
        config: SmartMonitoringCheckResultConfig,
        icinga2_client_config: Icinga2ClientConfig,
        director_client_config: DirectorClientConfig,
    ) -> Result<SmartMonitoringExecutor, ExecutorError> {
        Ok(SmartMonitoringExecutor {
            config,
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

    fn set_state_with_retry(
        icinga_executor: Arc<Icinga2Executor>,
        icinga2_action: Icinga2ActionOwned,
        host_name: Option<String>,
        service_name: Option<String>,
        total_attempts: u32,
        performed_attempts: u32,
        sleep_ms_between_retries: u64,
    ) -> Pin<Box<dyn Future<Output = Result<(), ExecutorError>>>> {
        Box::pin(async move {
            match icinga_executor.perform_request(&(&icinga2_action).into()).await {
                Ok(()) => {
                    trace!("SmartMonitoringExecutor - process_check_result for object host [{:?}] service [{:?}] successfully performed.", host_name, service_name);
                }
                Err(err) => {
                    warn!("SmartMonitoringExecutor - process_check_result for object host [{:?}] service [{:?}] completed with errors. err: {:?}", host_name, service_name, err);
                }
            }

            // check status
            let response = match (&host_name, &service_name) {
                (Some(host_name), Some(service_name)) => {
                    debug!(
                        "SmartMonitoringExecutor - check host [{}] service [{}] status",
                        host_name, service_name
                    );
                    icinga_executor
                        .api_client
                        .api_get_objects_service(host_name, service_name)
                        .await
                }
                (Some(host_name), None) => {
                    debug!("SmartMonitoringExecutor - check host [{}] status", host_name);
                    icinga_executor.api_client.api_get_objects_host(host_name).await
                }
                _ => {
                    warn!("SmartMonitoringExecutor - cannot identify host or service name to retry process_check_result");
                    Err(ExecutorError::ActionExecutionError { message: "SmartMonitoringExecutor - Cannot identify host or service name to retry process_check_result".to_owned(), can_retry: false, code: None, data: Default::default() })
                }
            }?;

            let url = response.url;
            let method = response.method;

            let response_json = response.response.json().await.map_err(|err| {
                ExecutorError::ActionExecutionError {
                    can_retry: true,
                    message: format!(
                        "SmartMonitoringExecutor - Cannot extract response body. Err: {:?}",
                        err
                    ),
                    code: None,
                    data: hashmap![
                        "url" => url.into(),
                        "method" => method.into()
                    ]
                    .into(),
                }
            })?;

            match SmartMonitoringExecutor::is_pending(&response_json) {
                Ok(false) => Ok(()),
                _ => {
                    if (total_attempts - performed_attempts) > 0 {
                        let new_performed_attempts = performed_attempts + 1;
                        info!("SmartMonitoringExecutor - the object host [{:?}] service [{:?}] is found to be pending or the state cannot be determined. Retrying to set the status. Retries: {} out of {}", host_name, service_name, new_performed_attempts, total_attempts);
                        tokio::time::sleep(Duration::from_millis(sleep_ms_between_retries)).await;
                        SmartMonitoringExecutor::set_state_with_retry(
                            icinga_executor,
                            icinga2_action,
                            host_name,
                            service_name,
                            total_attempts,
                            new_performed_attempts,
                            sleep_ms_between_retries,
                        )
                        .await
                    } else {
                        Err(ExecutorError::ActionExecutionError { message: format!("The object host [{:?}] service [{:?}] is found to be pending after {} attempts and no more attempts to set the status will be performed.", host_name, service_name, total_attempts), can_retry: true, code: None, data: Default::default() })
                    }
                }
            }
        })
    }

    /// Returns whether an object is pending.
    /// An object is pending if `last_check_result` is null.
    fn is_pending(icinga_object_query_response: &Value) -> Result<bool, ExecutorError> {
        trace!(
            "SmartMonitoringExecutor - icinga_object_query_response is {}",
            icinga_object_query_response
        );

        icinga_object_query_response.get("results")
            .and_then(|results| results.as_array())
            .and_then(|results| results.get(0))
            .and_then(|result| result.get("attrs"))
            .and_then(|attrs| attrs.get("last_check_result"))
            .map(|last_check_result| last_check_result.is_null())
            .ok_or_else(||
                ExecutorError::ActionExecutionError { message: "SmartMonitoringExecutor - Cannot determine whether the object is in pending state".to_owned(), can_retry: false, code: None, data: Default::default(), }
            )
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
            Err(ExecutorError::ActionExecutionError {
                message,
                code: Some(code),
                data: _,
                can_retry: _,
            }) if code.eq(ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE) => {
                debug!("SmartMonitoringExecutor - Process check result action failed with message {:?}. Looks like Icinga2 object does not exist yet. Proceeding with the creation of the object..", message);
                self.perform_creation_of_icinga_objects(
                    director_host_creation_action,
                    director_service_creation_action,
                )
                .await?;

                SmartMonitoringExecutor::set_state_with_retry(
                    self.icinga_executor.clone(),
                    Icinga2ActionOwned {
                        name: icinga2_action.name.to_owned(),
                        payload: icinga2_action.payload.cloned(),
                    },
                    host_name,
                    service_name,
                    self.config.pending_object_set_status_retries_attempts,
                    0,
                    self.config.pending_object_set_status_retries_sleep_ms,
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
    use serde_json::json;
    use tornado_common_api::{Action, Map, Value};

    #[tokio::test]
    async fn should_fail_if_action_data_is_missing() {
        // Arrange
        let executor = SmartMonitoringExecutor::new(
            Default::default(),
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
            _ => assert!(false),
        }
    }

    #[tokio::test]
    async fn should_return_ok_if_action_is_valid() {
        // Arrange
        let mock_server = MockServer::start();

        mock_server.mock(|when, then| {
            when.method(POST).path("/v1/actions/process-check-result");
            then.status(200);
        });

        let executor = SmartMonitoringExecutor::new(
            Default::default(),
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
            Default::default(),
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
            _ => assert!(false),
        }
    }

    #[tokio::test]
    async fn should_throw_error_if_host_name_missing() {
        // Arrange
        let executor = SmartMonitoringExecutor::new(
            Default::default(),
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
            _ => assert!(false),
        }
    }

    #[tokio::test]
    async fn should_throw_error_if_service_name_missing() {
        // Arrange
        let executor = SmartMonitoringExecutor::new(
            Default::default(),
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
            _ => assert!(false),
        }
    }

    #[tokio::test]
    async fn should_return_ok_if_action_type_is_host_and_service_creation_payload_not_given() {
        // Arrange
        let mock_server = MockServer::start();

        mock_server.mock(|when, then| {
            when.method(POST).path("/v1/actions/process-check-result");
            then.status(200);
        });

        let executor = SmartMonitoringExecutor::new(
            Default::default(),
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

    #[tokio::test]
    async fn should_return_state_check_pending() {
        // Arrange
        let icinga_response: serde_json::Value = serde_json::from_str(
            r#"
{
  "results": [
    {
      "attrs": {
        "__name": "ALCATEL-a360!ALCATEL-vrIFD_a360/r01sr1sl23/port#38-#3-7-3-Tu12-TMi",
        "last_check": -1,
        "last_check_result": null,
        "last_hard_state": 3,
        "last_hard_state_change": 0,
        "last_reachable": true,
        "last_state": 3,
        "last_state_change": 0,
        "state": 3,
        "state_type": 0,
        "type": "Service",
        "version": 0,
        "volatile": true,
        "zone": "master"
      },
      "joins": {},
      "meta": {},
      "name": "ALCATEL-a360!ALCATEL-vrIFD_a360/r01sr1sl23/port#38-#3-7-3-Tu12-TMi",
      "type": "Service"
    }
  ]
}
        "#,
        )
        .unwrap();

        // Act
        let is_pending = SmartMonitoringExecutor::is_pending(&icinga_response).unwrap();

        // Assert
        assert!(is_pending);
    }

    #[test]
    fn should_return_state_check_not_pending() {
        // Arrange
        let icinga_response: serde_json::Value = serde_json::from_str(r#"
{
  "results": [
    {
      "attrs": {
        "__name": "test03!service_test03",
        "last_check": 1611653536.602431,
        "last_check_result": {
          "active": true,
          "check_source": "713182e2afcb",
          "command": [
            "/sbin/neteye",
            "check"
          ],
          "execution_end": 1611653536.602379,
          "execution_start": 1611653535.630243,
          "exit_status": 3,
          "output": "UNKNOWN - At least one health check is in unknown state.\n\n[+] light/00100_neteye_target_enabled.sh\n[+] light/00200_drbd_status.sh\n[-] light/01000_elastic_health_check.sh\n[-] Something went wrong in contacting Elasticsearch\n[-] Error: \n[-] Exit code of curl: 35\n[-] light/01001_elastic_indices_check.sh\n[-] Elasticsearch API (cluster health) not reachable\n[-] light/01002_elastic_indices_read_only_check.sh\n[-] Elasticsearch API (settings) not reachable\n[+] light/01003_elastic_nodes_health_check.sh\n[+] light/01010_service_assetmanagement_glpi_roles_fullentities_map_enabled_post_4.14.sh\n[+] light/01020_tornado_tcp_is_enabled.sh\n[+] light/01030_logstash_health_check.sh\n[+] light/01031_check_logstash_user_health_check.sh\n[+] light/01203_analytics_grafana_user_check.sh\n[+] light/01210_analytics_grafana_sync_enabled.sh\n[+] light/01500_log_manager_log_check_light.sh\n[+] light/01600_neteye_retentionpolicy_enabled.sh\n[+] light/01800_ntopng_sync_enabled.sh",
          "performance_data": [],
          "schedule_end": 1611653536.602431,
          "schedule_start": 1611653535.63,
          "state": 3,
          "ttl": 0,
          "type": "CheckResult",
          "vars_after": {
            "attempt": 1,
            "reachable": false,
            "state": 3,
            "state_type": 1
          },
          "vars_before": {
            "attempt": 1,
            "reachable": false,
            "state": 3,
            "state_type": 1
          }
        },
        "last_hard_state": 3,
        "last_hard_state_change": 1611648919.80426,
        "last_reachable": false,
        "last_state": 3,
        "last_state_change": 1611648803.615403,
        "last_state_critical": 0,
        "last_state_ok": 1611596478.584076,
        "last_state_type": 1,
        "state": 3,
        "state_type": 1,
        "type": "Service",
        "vars": null,
        "version": 0,
        "volatile": false,
        "zone": "master"
      },
      "joins": {},
      "meta": {},
      "name": "test03!service_test03",
      "type": "Service"
    }
  ]
}
        "#).unwrap();

        // Act
        let is_pending = SmartMonitoringExecutor::is_pending(&icinga_response).unwrap();

        // Assert
        assert!(!is_pending);
    }
}
