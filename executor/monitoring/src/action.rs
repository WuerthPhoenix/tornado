use serde::{Deserialize, Serialize};
use tornado_common_api::{Action, Payload, Value};
use tornado_executor_common::ExecutorError;
use tornado_executor_director::{DirectorAction, DirectorActionName};
use tornado_executor_icinga2::Icinga2Action;

const PROCESS_CHECK_RESULT_SUBURL: &str = "process-check-result";
pub const ICINGA_FIELD_FOR_SPECIFYING_HOST: &str = "host";
pub const ICINGA_FIELD_FOR_SPECIFYING_SERVICE: &str = "service";

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
    pub fn to_sub_actions(
        &mut self,
    ) -> Result<(Icinga2Action, DirectorAction, Option<DirectorAction>), ExecutorError> {
        match self {
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
                }
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
            MonitoringAction::SimpleCreateAndProcess { check_result, host, service } => {

                host.insert("object_type".to_owned(), Value::Text("Object".to_owned()));
                let host_object_name = host.get("object_name").and_then(|value| value.get_text()).ok_or_else(||
                    ExecutorError::ConfigurationError { message: format!("Monitoring action expects that field '{}' inside '{}' for action '{}' is of type text", "object_name", "host", "simple_create_and_or_process_passive_check_result" ) }
                )?;

                let create_host_director_action = DirectorAction {
                    name: DirectorActionName::CreateHost,
                    payload: host,
                    live_creation: true,
                };

                check_result.insert("type".to_owned(), Value::Text("Host".to_owned()));
                check_result.insert("host".to_owned(), Value::Text(host_object_name.to_owned()));

                Ok((Icinga2Action {
                    name: PROCESS_CHECK_RESULT_SUBURL,
                    payload: Some(check_result),
                }, DirectorAction {
                    name: DirectorActionName::CreateHost,
                    payload: host,
                    live_creation: true,
                }, None))
            }
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use maplit::*;
    use std::collections::HashMap;
    use tornado_common_api::Value;

    #[test]
    fn to_sub_actions_should_throw_error_if_process_check_result_host_not_specified_with_host_field(
    ) {
        // Arrange
        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::Text("create_and_or_process_host_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "filter".to_owned() => Value::Text("host.name==\"myhost\"".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));

        let mut monitoring_action = MonitoringAction::new(&action).unwrap();

        // Act
        let result = monitoring_action.to_sub_actions();
        println!("{:?}", result);

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("Monitoring action expects that Icinga objects affected by the action are specified with field 'host'"))
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn to_sub_actions_should_throw_error_if_process_check_result_service_not_specified_with_service_field(
    ) {
        // Arrange
        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::Text("create_and_or_process_service_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "filter".to_owned() => Value::Text("host.name==\"myhost\"".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));
        action.payload.insert("service_creation_payload".to_owned(), Value::Map(HashMap::new()));

        let mut monitoring_action = MonitoringAction::new(&action).unwrap();

        // Act
        let result = monitoring_action.to_sub_actions();

        println!("{:?}", result);

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("Monitoring action expects that Icinga objects affected by the action are specified with field 'service'"))
            }
            _ => assert!(false),
        }
    }

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
    fn should_parse_a_simple_create_and_or_process_passive_check_result_action_for_a_host() {
        // Arrange
        let filename =
            "./tests_resources/simple_create_and_or_process_passive_check_result_host.json";
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

    #[test]
    fn should_parse_a_simple_create_and_or_process_passive_check_result_action_for_a_service() {
        // Arrange
        let filename =
            "./tests_resources/simple_create_and_or_process_passive_check_result_service.json";
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

    #[test]
    fn simple_create_should_be_equivalent_to_full_create_host_01() {
        // Arrange
        let filename_full = "./tests_resources/monitoring_host_01_full.json";
        let filename_simple = "./tests_resources/monitoring_host_01_simple.json";

        let action_full: Action = serde_json::from_str(&std::fs::read_to_string(filename_full)
            .expect(&format!("Unable to open the file [{}]", filename_full))).unwrap();

        let action_simple: Action = serde_json::from_str(&std::fs::read_to_string(filename_simple)
            .expect(&format!("Unable to open the file [{}]", filename_simple))).unwrap();

        // Act
        let mut monitoring_action_full = MonitoringAction::new(&action_full).unwrap();
        let sub_actions_full = monitoring_action_full.to_sub_actions().unwrap();

        let mut monitoring_action_simple = MonitoringAction::new(&action_simple).unwrap();
        let sub_actions_simple = monitoring_action_simple.to_sub_actions().unwrap();
        // Assert
        assert_eq!(sub_actions_full, sub_actions_simple)
    }
}
