use log::*;
use serde::{Deserialize, Serialize};
use tornado_common_api::Payload;
use tornado_common_api::{Action, Value};
use tornado_executor_common::{Executor, ExecutorError};

pub const MONITORING_ACTION_NAME_KEY: &str = "action_name";
pub const ICINGA_FIELD_FOR_SPECIFYING_HOST: &str = "host";
pub const ICINGA_FIELD_FOR_SPECIFYING_SERVICE: &str = "service";

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "action_name")]
pub enum MonitoringAction {
    #[serde(rename = "create_and_or_process_host_passive_check_result")]
    Host {
        process_check_result_payload: Payload,
        host_creation_payload: Value,
        #[serde(default = "live_creation_default")]
        icinga2_live_creation: bool,
    },
    #[serde(rename = "create_and_or_process_service_passive_check_result")]
    Service {
        process_check_result_payload: Payload,
        host_creation_payload: Value,
        service_creation_payload: Value,
        #[serde(default = "live_creation_default")]
        icinga2_live_creation: bool,
    },
}

fn live_creation_default() -> bool {
    false
}

/// An executor that prepares the Monitoring action and sends it to the Monitoring Api Client executor
#[derive(Default)]
pub struct MonitoringExecutor<F: Fn(MonitoringAction) -> Result<(), ExecutorError>> {
    callback: F,
}

impl<F: Fn(MonitoringAction) -> Result<(), ExecutorError>> std::fmt::Display
    for MonitoringExecutor<F>
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("MonitoringExecutor")?;
        Ok(())
    }
}

impl<F: Fn(MonitoringAction) -> Result<(), ExecutorError>> MonitoringExecutor<F> {
    pub fn new(callback: F) -> MonitoringExecutor<F> {
        MonitoringExecutor { callback }
    }
}

impl<F: Fn(MonitoringAction) -> Result<(), ExecutorError>> Executor for MonitoringExecutor<F> {
    fn execute(&mut self, action: Action) -> Result<(), ExecutorError> {
        trace!("MonitoringExecutor - received action: \n[{:?}]", action);

        // TODO: do I really have to convert the Action to serde value and then back to MonitoringAction, in order to transform with serde?
        let monitoring_action: MonitoringAction =
            serde_json::to_value(tornado_common_api::Value::Map(action.payload))
                .and_then(serde_json::from_value)
                .map_err(|err| ExecutorError::ConfigurationError {
                    message: format!("Invalid Monitoring Action configuration. Err: {}", err),
                })?;

        // we need to be sure that the icinga2 action specifies the object on which to apply the action
        // with the fields "host" or "service", and not, e.g. with "filter"
        match &monitoring_action {
            MonitoringAction::Host { process_check_result_payload, .. } => {
                if process_check_result_payload.get(ICINGA_FIELD_FOR_SPECIFYING_HOST).is_none() {
                    return Err(ExecutorError::ConfigurationError { message: format!("Monitoring action expects that Icinga objects affected by the action are specified with field '{}' inside '{}' for action '{}'", ICINGA_FIELD_FOR_SPECIFYING_HOST, "process_check_result_payload", "create_and_or_process_host_passive_check_result" ) });
                }
            }
            MonitoringAction::Service { process_check_result_payload, .. } => {
                if process_check_result_payload.get(ICINGA_FIELD_FOR_SPECIFYING_SERVICE).is_none() {
                    return Err(ExecutorError::ConfigurationError { message: format!("Monitoring action expects that Icinga objects affected by the action are specified with field '{}' inside '{}' for action '{}'", ICINGA_FIELD_FOR_SPECIFYING_SERVICE, "process_check_result_payload", "create_and_or_process_service_passive_check_result" ) });
                }
            }
        };

        (self.callback)(monitoring_action)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use maplit::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tornado_common_api::Value;

    #[test]
    fn should_fail_if_action_missing() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));

        let mut executor = MonitoringExecutor::new(|monitoring_action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(monitoring_action);
            Ok(())
        });

        let action = Action::new("");

        // Act
        let result = executor.execute(action);

        // Assert
        assert!(result.is_err());
        assert_eq!(
            Err(ExecutorError::ConfigurationError {
                message:
                    "Invalid Monitoring Action configuration. Err: missing field `action_name`"
                        .to_owned()
            }),
            result
        );
        assert_eq!(None, *callback_called.lock().unwrap());
    }

    #[test]
    fn should_throw_error_if_action_name_is_not_valid() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));
        let mut executor = MonitoringExecutor::new(|monitoring_action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(monitoring_action);
            Ok(())
        });

        let mut action = Action::new("");
        action
            .payload
            .insert("action_name".to_owned(), Value::Text("my_invalid_action".to_owned()));
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "host".to_owned() => Value::Text("myhost".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));
        action.payload.insert("service_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(action);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_throw_error_if_service_action_but_service_creation_payload_not_given() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));
        let mut executor = MonitoringExecutor::new(|monitoring_action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(monitoring_action);
            Ok(())
        });

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::Text("create_and_or_process_service_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "service".to_owned() => Value::Text("myservice".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(action);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_return_ok_if_action_name_is_valid() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));
        let mut executor = MonitoringExecutor::new(|monitoring_action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(monitoring_action);
            Ok(())
        });

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::Text("create_and_or_process_host_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "host".to_owned() => Value::Text("myhost".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));
        action.payload.insert("service_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(action);

        println!("{:?}", result);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn should_throw_error_if_process_check_result_host_not_specified_with_host_field() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));
        let mut executor = MonitoringExecutor::new(|monitoring_action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(monitoring_action);
            Ok(())
        });

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

        // Act
        let result = executor.execute(action);

        println!("{:?}", result);

        // Assert
        assert!(result.is_err());
        assert_eq!(result, Err(ExecutorError::ConfigurationError { message: "Monitoring action expects that Icinga objects affected by the action are specified with field 'host' inside 'process_check_result_payload' for action 'create_and_or_process_host_passive_check_result'".to_string() }))
    }

    #[test]
    fn should_throw_error_if_process_check_result_service_not_specified_with_service_field() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));
        let mut executor = MonitoringExecutor::new(|monitoring_action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(monitoring_action);
            Ok(())
        });

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

        // Act
        let result = executor.execute(action);

        println!("{:?}", result);

        // Assert
        assert!(result.is_err());
        assert_eq!(result, Err(ExecutorError::ConfigurationError { message: "Monitoring action expects that Icinga objects affected by the action are specified with field 'service' inside 'process_check_result_payload' for action 'create_and_or_process_service_passive_check_result'".to_string() }))
    }


    #[test]
    fn should_return_ok_if_action_type_is_host_and_service_creation_payload_not_given() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));
        let mut executor = MonitoringExecutor::new(|monitoring_action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(monitoring_action);
            Ok(())
        });

        let mut action = Action::new("");
        action.payload.insert(
            "action_name".to_owned(),
            Value::Text("create_and_or_process_host_passive_check_result".to_owned()),
        );
        action.payload.insert(
            "process_check_result_payload".to_owned(),
            Value::Map(hashmap!(
                "host".to_owned() => Value::Text("myhost".to_owned()),
            )),
        );
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(action);

        println!("{:?}", result);

        // Assert
        assert!(result.is_ok());
    }
}
