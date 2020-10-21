use serde::{Deserialize, Serialize};
use tornado_common_api::{Action, Payload, Value};
use tornado_executor_common::ExecutorError;
use tornado_executor_director::{DirectorAction, DirectorActionName};
use tornado_executor_icinga2::Icinga2Action;

const PROCESS_CHECK_RESULT_SUBURL: &str = "process-check-result";

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "action_name")]
pub enum MonitoringAction {
    #[serde(rename = "create_and_or_process_host_passive_check_result")]
    Host { process_check_result_payload: Payload, host_creation_payload: Value },
    #[serde(rename = "create_and_or_process_service_passive_check_result")]
    Service {
        process_check_result_payload: Payload,
        host_creation_payload: Value,
        service_creation_payload: Value,
    },
    #[serde(rename = "simple_create_and_or_process_passive_check_result")]
    SimpleCreateAndProcess { check_result: Payload, host: Payload, service: Option<Payload> },
}

impl MonitoringAction {
    pub fn new(action: &Action) -> Result<MonitoringAction, ExecutorError> {
        Ok(serde_json::to_value(&action.payload).and_then(serde_json::from_value).map_err(
            |err| ExecutorError::ConfigurationError {
                message: format!("Invalid Monitoring Action configuration. Err: {}", err),
            },
        )?)
    }

    // Transforms the MonitoringAction into the actions needed to call the IcingaExecutor and the
    // DirectorExecutor.
    // Returns a triple, with these elements:
    // 1. Icinga2Action that will perform the process-check-result through the IcingaExecutor
    // 2. DirectorAction that will perform the creation of the host through the DirectorAction
    // 3. Option<DirectorAction> that will perform the creation of the service through the
    // DirectorAction. This is Some if MonitoringAction is of type Service, None otherwise
    pub fn to_sub_actions(&self) -> (Icinga2Action, DirectorAction, Option<DirectorAction>) {
        match &self {
            MonitoringAction::Host { process_check_result_payload, host_creation_payload } => (
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
            ),
            MonitoringAction::Service {
                process_check_result_payload,
                host_creation_payload,
                service_creation_payload,
            } => (
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
            ),
            MonitoringAction::SimpleCreateAndProcess { check_result, host, service } => {
                let remove_me = 0;
                unimplemented!()
            }
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_parse_a_create_and_or_process_service_passive_check_result_action() {
        // Arrange
        let filename = "./tests_resources/create_and_or_process_service_passive_check_result.json";
        let json = std::fs::read_to_string(filename)
            .expect(&format!("Unable to open the file [{}]", filename));
        let action: Action = serde_json::from_str(&json).unwrap();

        // Act
        let action = MonitoringAction::new(&action).unwrap();

        // Assert
        match action {
            MonitoringAction::Service { .. } => {}
            _ => assert!(false),
        }
    }

    #[test]
    fn should_parse_a_simple_create_and_or_process_passive_check_result_action() {
        // Arrange
        let filename = "./tests_resources/simple_create_and_or_process_passive_check_result.json";
        let json = std::fs::read_to_string(filename)
            .expect(&format!("Unable to open the file [{}]", filename));
        let action: Action = serde_json::from_str(&json).unwrap();

        // Act
        let action = MonitoringAction::new(&action).unwrap();

        // Assert
        match action {
            MonitoringAction::SimpleCreateAndProcess { .. } => {}
            _ => assert!(false),
        }
    }
}
