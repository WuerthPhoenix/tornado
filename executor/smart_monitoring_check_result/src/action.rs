use serde::{Deserialize, Serialize};
use serde_json::json;
use tornado_common_api::{Action, Payload, Value, ValueExt};
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
    pub fn new(action: &Action) -> Result<Self, ExecutorError> {
        let mut scap: Self = serde_json::to_value(&action.payload)
            .and_then(serde_json::from_value)
            .map_err(|err| ExecutorError::ConfigurationError {
                message: format!(
                    "Invalid SimpleCreateAndProcess Action configuration. Err: {:?}",
                    err
                ),
            })?;

        let created_ms = action.created_ms as f32 / 1000.0;
        scap.check_result.entry("execution_start").or_insert(json!(created_ms));
        scap.check_result.entry("execution_end").or_insert(json!(created_ms));

        Ok(scap)
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
            Value::String("Object".to_owned()),
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
                Value::String("Object".to_owned()),
            );
            service_payload.insert(
                ICINGA_FIELD_FOR_SPECIFYING_HOST.to_owned(),
                Value::String(host_object_name.to_owned()),
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
                Value::String("Service".to_owned()),
            );
            self.check_result.insert(
                ICINGA_FIELD_FOR_SPECIFYING_SERVICE.to_owned(),
                Value::String(format!("{}!{}", host_object_name, service_object_name)),
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
                Value::String("Host".to_owned()),
            );
            self.check_result.insert(
                ICINGA_FIELD_FOR_SPECIFYING_HOST.to_owned(),
                Value::String(host_object_name.to_owned()),
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
    use serde_json::json;
    use tornado_common_api::{Action, Map, Value};
    use tornado_engine_matcher::config::rule::ConfigAction;

    #[test]
    fn to_sub_actions_should_throw_error_if_process_check_result_host_not_specified_with_host_field(
    ) {
        // Arrange
        let mut action = Action::new("");
        action.payload.insert("check_result".to_owned(), Value::Object(Map::new()));
        action.payload.insert("host".to_owned(), Value::Object(Map::new()));
        action.payload.insert(
            "service".to_owned(),
            json!(hashmap!(
                "object_name".to_owned() => Value::String("myservice".to_owned()),
            )),
        );

        let mut monitoring_action = SimpleCreateAndProcess::new(&action).unwrap();

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
        let mut action = Action::new("");
        action.payload.insert("check_result".to_owned(), Value::Object(Map::new()));
        action.payload.insert(
            "host".to_owned(),
            json!(hashmap!(
                "object_name".to_owned() => Value::String("myhost".to_owned()),
            )),
        );
        action.payload.insert("service".to_owned(), Value::Object(Map::new()));

        let mut monitoring_action = SimpleCreateAndProcess::new(&action).unwrap();

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
        let action: Action = serde_json::from_str(&json).unwrap();

        // Act
        let action = SimpleCreateAndProcess::new(&action).unwrap();

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
        let action = serde_json::from_str(&json).unwrap();

        // Act
        let action = SimpleCreateAndProcess::new(&action).unwrap();

        // Assert
        assert!(action.service.is_some());
    }

    #[derive(Debug, PartialEq, Deserialize)]
    pub struct MonitoringHostData {
        process_check_result_payload: Payload,
        host_creation_payload: Payload,
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
