use serde::{Deserialize, Serialize};
use tornado_common_api::{Payload, Value};
use tornado_executor_common::ExecutorError;
use tornado_executor_director::{DirectorAction, DirectorActionName};
use tornado_executor_icinga2::Icinga2Action;

const PROCESS_CHECK_RESULT_SUBURL: &str = "process-check-result";
pub const ICINGA_FIELD_FOR_SPECIFYING_HOST: &str = "host";
pub const ICINGA_FIELD_FOR_SPECIFYING_SERVICE: &str = "service";
pub const ICINGA_FIELD_FOR_SPECIFYING_TYPE: &str = "type";
pub const ICINGA_FIELD_FOR_SPECIFYING_OBJECT_TYPE: &str = "object_type";
pub const ICINGA_FIELD_FOR_SPECIFYING_OBJECT_NAME: &str = "object_name";

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct SimpleCreateAndProcess {
    check_result: Payload,
    host: Payload,
    service: Option<Payload>,
}

impl SimpleCreateAndProcess {
    pub fn new(payload: &Payload) -> Result<SimpleCreateAndProcess, ExecutorError> {
        serde_json::to_value(payload).and_then(serde_json::from_value).map_err(|err| {
            ExecutorError::ConfigurationError {
                message: format!(
                    "Invalid SimpleCreateAndProcess Action configuration. Err: {:?}",
                    err
                ),
            }
        })
    }

    // Transforms the SimpleCreateAndProcess into the actions needed to call the IcingaExecutor and the
    // DirectorExecutor.
    // Returns a triple, with these elements:
    // 1. Icinga2Action that will perform the process-check-result through the IcingaExecutor
    // 2. DirectorAction that will perform the creation of the host through the DirectorAction
    // 3. Option<DirectorAction> that will perform the creation of the service through the
    // DirectorAction. This is Some if SimpleCreateAndProcess is of type Service, None otherwise
    pub fn build_sub_actions(
        &mut self,
    ) -> Result<(Icinga2Action, DirectorAction, Option<DirectorAction>), ExecutorError> {
        self.host.insert(
            ICINGA_FIELD_FOR_SPECIFYING_OBJECT_TYPE.to_owned(),
            Value::Text("Object".to_owned()),
        );
        let host_object_name = self
            .host
            .get(ICINGA_FIELD_FOR_SPECIFYING_OBJECT_NAME)
            .and_then(|value| value.get_text())
            .ok_or_else(|| ExecutorError::ConfigurationError {
                message: format!(
                    "Monitoring action expects that field '{}' inside '{}'",
                    ICINGA_FIELD_FOR_SPECIFYING_OBJECT_NAME, ICINGA_FIELD_FOR_SPECIFYING_HOST
                ),
            })?;

        let director_create_host_action = DirectorAction {
            name: DirectorActionName::CreateHost,
            payload: &self.host,
            live_creation: true,
        };

        if let Some(service_payload) = &mut self.service {
            service_payload.insert(
                ICINGA_FIELD_FOR_SPECIFYING_OBJECT_TYPE.to_owned(),
                Value::Text("Object".to_owned()),
            );
            service_payload.insert(
                ICINGA_FIELD_FOR_SPECIFYING_HOST.to_owned(),
                Value::Text(host_object_name.to_owned()),
            );
            let service_object_name = service_payload
                .get(ICINGA_FIELD_FOR_SPECIFYING_OBJECT_NAME)
                .and_then(|value| value.get_text())
                .ok_or_else(|| ExecutorError::ConfigurationError {
                    message: format!(
                        "Monitoring action expects that field '{}' inside '{}'",
                        ICINGA_FIELD_FOR_SPECIFYING_OBJECT_NAME,
                        ICINGA_FIELD_FOR_SPECIFYING_SERVICE
                    ),
                })?;
            self.check_result.insert(
                ICINGA_FIELD_FOR_SPECIFYING_TYPE.to_owned(),
                Value::Text("Service".to_owned()),
            );
            self.check_result.insert(
                ICINGA_FIELD_FOR_SPECIFYING_SERVICE.to_owned(),
                Value::Text(format!("{}!{}", host_object_name, service_object_name)),
            );
            Ok((
                Icinga2Action {
                    name: PROCESS_CHECK_RESULT_SUBURL,
                    payload: Some(&self.check_result),
                },
                director_create_host_action,
                Some(DirectorAction {
                    name: DirectorActionName::CreateService,
                    payload: service_payload,
                    live_creation: true,
                }),
            ))
        } else {
            self.check_result.insert(
                ICINGA_FIELD_FOR_SPECIFYING_TYPE.to_owned(),
                Value::Text("Host".to_owned()),
            );
            self.check_result.insert(
                ICINGA_FIELD_FOR_SPECIFYING_HOST.to_owned(),
                Value::Text(host_object_name.to_owned()),
            );
            Ok((
                Icinga2Action {
                    name: PROCESS_CHECK_RESULT_SUBURL,
                    payload: Some(&self.check_result),
                },
                director_create_host_action,
                None,
            ))
        }
    }

    pub fn get_host_name(&self) -> Option<&str> {
        self.host.get(ICINGA_FIELD_FOR_SPECIFYING_OBJECT_NAME).and_then(|value| value.get_text())
    }

    pub fn get_service_name(&self) -> Option<&str> {
        self.service
            .as_ref()
            .and_then(|service| service.get(ICINGA_FIELD_FOR_SPECIFYING_OBJECT_NAME))
            .and_then(|value| value.get_text())
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use maplit::*;
    use tornado_common_api::{Action, Value};

    #[test]
    fn to_sub_actions_should_throw_error_if_process_check_result_host_not_specified_with_host_field(
    ) {
        // Arrange
        let mut action = Action::new("", "");
        action.payload.insert("check_result".to_owned(), Value::Map(hashmap!()));
        action.payload.insert("host".to_owned(), Value::Map(hashmap!()));
        action.payload.insert(
            "service".to_owned(),
            Value::Map(hashmap!(
                "object_name".to_owned() => Value::Text("myservice".to_owned()),
            )),
        );

        let mut monitoring_action = SimpleCreateAndProcess::new(&action.payload).unwrap();

        // Act
        let result = monitoring_action.build_sub_actions();
        println!("{:?}", result);

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("host"));
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn to_sub_actions_should_throw_error_if_process_check_result_service_not_specified_with_service_field(
    ) {
        // Arrange
        let mut action = Action::new("", "");
        action.payload.insert("check_result".to_owned(), Value::Map(hashmap!()));
        action.payload.insert(
            "host".to_owned(),
            Value::Map(hashmap!(
                "object_name".to_owned() => Value::Text("myhost".to_owned()),
            )),
        );
        action.payload.insert("service".to_owned(), Value::Map(hashmap!()));

        let mut monitoring_action = SimpleCreateAndProcess::new(&action.payload).unwrap();

        // Act
        let result = monitoring_action.build_sub_actions();

        println!("{:?}", result);

        // Assert
        match result {
            Err(ExecutorError::ConfigurationError { message }) => {
                assert!(message.contains("service"))
            }
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
        let action: tornado_engine_matcher::config::rule::Action = serde_json::from_str(&json).unwrap();

        // Act
        let action = SimpleCreateAndProcess::new(&action.payload).unwrap();

        // Assert
        assert!(action.service.is_none());
    }

    #[test]
    fn should_parse_a_simple_create_and_or_process_passive_check_result_action_for_a_service() {
        // Arrange
        let filename =
            "./tests_resources/simple_create_and_or_process_passive_check_result_service.json";
        let json = std::fs::read_to_string(filename)
            .expect(&format!("Unable to open the file [{}]", filename));
        let action: tornado_engine_matcher::config::rule::Action = serde_json::from_str(&json).unwrap();

        // Act
        let action = SimpleCreateAndProcess::new(&action.payload).unwrap();

        // Assert
        assert!(action.service.is_some());
    }

    #[test]
    fn simple_create_should_be_equivalent_to_full_create_host_01() {
        // Arrange
        let filename_full = "./tests_resources/monitoring_host_01_full.json";
        let filename_simple = "./tests_resources/monitoring_host_01_simple.json";

        let monitoring_action_full: MonitoringHostData = serde_json::from_str(
            &std::fs::read_to_string(filename_full)
                .expect(&format!("Unable to open the file [{}]", filename_full)),
        )
        .unwrap();

        let action_simple: tornado_engine_matcher::config::rule::Action = serde_json::from_str(
            &std::fs::read_to_string(filename_simple)
                .expect(&format!("Unable to open the file [{}]", filename_simple)),
        )
        .unwrap();

        // Act
        let sub_actions_full = monitoring_action_full.to_sub_actions();

        let mut monitoring_action_simple =
            SimpleCreateAndProcess::new(&action_simple.payload).unwrap();
        let sub_actions_simple = monitoring_action_simple.build_sub_actions().unwrap();
        // Assert
        assert_eq!(sub_actions_full, sub_actions_simple)
    }

    #[test]
    fn simple_create_should_be_equivalent_to_full_create_service_01() {
        // Arrange
        let filename_full = "./tests_resources/monitoring_service_01_full.json";
        let filename_simple = "./tests_resources/monitoring_service_01_simple.json";

        let monitoring_action_full: MonitoringServiceData = serde_json::from_str(
            &std::fs::read_to_string(filename_full)
                .expect(&format!("Unable to open the file [{}]", filename_full)),
        )
        .unwrap();

        let action_simple: tornado_engine_matcher::config::rule::Action = serde_json::from_str(
            &std::fs::read_to_string(filename_simple)
                .expect(&format!("Unable to open the file [{}]", filename_simple)),
        )
        .unwrap();

        // Act
        let sub_actions_full = monitoring_action_full.to_sub_actions();

        let mut monitoring_action_simple =
            SimpleCreateAndProcess::new(&action_simple.payload).unwrap();
        let sub_actions_simple = monitoring_action_simple.build_sub_actions().unwrap();
        // Assert
        assert_eq!(sub_actions_full, sub_actions_simple)
    }

    #[derive(Debug, PartialEq, Deserialize)]
    pub struct MonitoringHostData {
        process_check_result_payload: Payload,
        host_creation_payload: Payload,
    }

    impl MonitoringHostData {
        fn to_sub_actions(&self) -> (Icinga2Action, DirectorAction, Option<DirectorAction>) {
            (
                Icinga2Action {
                    name: PROCESS_CHECK_RESULT_SUBURL,
                    payload: Some(&self.process_check_result_payload),
                },
                DirectorAction {
                    name: DirectorActionName::CreateHost,
                    payload: &self.host_creation_payload,
                    live_creation: true,
                },
                None,
            )
        }
    }

    #[derive(Debug, PartialEq, Deserialize)]
    pub struct MonitoringServiceData {
        process_check_result_payload: Payload,
        host_creation_payload: Payload,
        service_creation_payload: Payload,
    }

    impl MonitoringServiceData {
        fn to_sub_actions(&self) -> (Icinga2Action, DirectorAction, Option<DirectorAction>) {
            (
                Icinga2Action {
                    name: PROCESS_CHECK_RESULT_SUBURL,
                    payload: Some(&self.process_check_result_payload),
                },
                DirectorAction {
                    name: DirectorActionName::CreateHost,
                    payload: &self.host_creation_payload,
                    live_creation: true,
                },
                Some(DirectorAction {
                    name: DirectorActionName::CreateService,
                    payload: &self.service_creation_payload,
                    live_creation: true,
                }),
            )
        }
    }
}
