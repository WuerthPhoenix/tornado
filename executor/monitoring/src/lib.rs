use log::*;
use serde::{Serialize,Deserialize};
use tornado_common_api::Action;
use tornado_common_api::Payload;
use tornado_executor_common::{Executor, ExecutorError};

pub const MONITORING_ACTION_NAME_KEY: &str = "action_name";

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "action_name")]
pub enum MonitoringAction {
    #[serde(rename = "create_and_or_process_host_passive_check_result")]
    Host {
        process_check_result_payload: Payload,
        host_creation_payload: Payload,
        #[serde(default = "live_creation_default")]
        icinga2_live_creation: bool,
    },
    #[serde(rename = "create_and_or_process_service_passive_check_result")]
    Service {
        process_check_result_payload: Payload,
        host_creation_payload: Payload,
        service_creation_payload: Payload,
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
        let monitoring_action =
            serde_json::to_value(tornado_common_api::Value::Map(action.payload)).and_then(serde_json::from_value).map_err(|err| {
                ExecutorError::ConfigurationError {
                    message: format!("Invalid Monitoring Action configuration. Err: {}", err),
                }
            })?;

        (self.callback)(monitoring_action)
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tornado_common_api::Value;
    use std::collections::HashMap;

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
                message: "Invalid Monitoring Action configuration. Err: missing field `action_name`".to_owned()
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
        action.payload.insert(
            "action_name".to_owned(),
            Value::Text("my_invalid_action".to_owned()),
        );
        action.payload.insert("process_check_result_payload".to_owned(), Value::Map(HashMap::new()));
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
        action.payload.insert("process_check_result_payload".to_owned(), Value::Map(HashMap::new()));
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
        action.payload.insert("process_check_result_payload".to_owned(), Value::Map(HashMap::new()));
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));
        action.payload.insert("service_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(action);

        println!("{:?}", result);

        // Assert
        assert!(result.is_ok());
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
        action.payload.insert("process_check_result_payload".to_owned(), Value::Map(HashMap::new()));
        action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));

        // Act
        let result = executor.execute(action);

        println!("{:?}", result);

        // Assert
        assert!(result.is_ok());
    }

}
