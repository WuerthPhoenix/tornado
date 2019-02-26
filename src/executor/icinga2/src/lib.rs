use log::*;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use tornado_common_api::Action;
use tornado_common_api::Payload;
use tornado_common_api::Value;
use tornado_executor_common::{Executor, ExecutorError};

pub const ICINGA2_ACTION_NAME_KEY: &str = "icinga2_action_name";
pub const ICINGA2_ACTION_PAYLOAD_KEY: &str = "icinga2_action_payload";

/// An executor that logs received actions at the 'info' level
#[derive(Default)]
pub struct Icinga2Executor<F: Fn(Icinga2Action) -> Result<(), ExecutorError>> {
    callback: F,
}

impl<F: Fn(Icinga2Action) -> Result<(), ExecutorError>> std::fmt::Display for Icinga2Executor<F> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("Icinga2Executor")?;
        Ok(())
    }
}

impl<F: Fn(Icinga2Action) -> Result<(), ExecutorError>> Icinga2Executor<F> {
    pub fn new(callback: F) -> Icinga2Executor<F> {
        Icinga2Executor { callback }
    }

    fn get_payload(&self, payload: &Payload) -> HashMap<String, Value> {
        match payload.get(ICINGA2_ACTION_PAYLOAD_KEY).and_then(|value| value.get_map()) {
            Some(icinga2_payload) => icinga2_payload.clone(),
            None => HashMap::new(),
        }
    }
}

impl<F: Fn(Icinga2Action) -> Result<(), ExecutorError>> Executor for Icinga2Executor<F> {
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError> {
        debug!("Icinga2Executor - received action: \n[{:#?}]", action);

        match action.payload.get(ICINGA2_ACTION_NAME_KEY).and_then(|value| value.get_text()) {
            Some(icinga2_action) => {
                info!("Icinga2Executor - perform Icinga2Action: \n[{:#?}]", icinga2_action);

                let action_payload = self.get_payload(&action.payload);

                (self.callback)(Icinga2Action {
                    name: icinga2_action.to_owned(),
                    payload: action_payload,
                })
            }
            None => Err(ExecutorError::MissingArgumentError {
                message: "Icinga2 Action not specified".to_string(),
            }),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Icinga2Action {
    pub name: String,
    pub payload: Payload,
}

#[cfg(test)]
mod test {
    use super::*;
    use maplit::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tornado_common_api::Value;

    #[test]
    fn should_fail_if_action_missing() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));

        let mut executor = Icinga2Executor::new(|icinga2action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(icinga2action);
            Ok(())
        });

        let action = Action::new("");

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_err());
        assert_eq!(
            Err(ExecutorError::MissingArgumentError {
                message: "Icinga2 Action not specified".to_owned()
            }),
            result
        );
        assert_eq!(None, *callback_called.lock().unwrap());
    }

    #[test]
    fn should_have_empty_payload_if_action_does_not_contains_one() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));
        let mut executor = Icinga2Executor::new(|icinga2action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(icinga2action);
            Ok(())
        });

        let mut action = Action::new("");
        action
            .payload
            .insert(ICINGA2_ACTION_NAME_KEY.to_owned(), Value::Text("action-test".to_owned()));

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_ok());
        assert_eq!(
            Some(Icinga2Action { name: "action-test".to_owned(), payload: HashMap::new() }),
            *callback_called.lock().unwrap()
        );
    }

    #[test]
    fn should_call_the_callback_if_valid_action() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));
        let mut executor = Icinga2Executor::new(|icinga2action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(icinga2action);
            Ok(())
        });

        let mut action = Action::new("");
        action.payload.insert(
            ICINGA2_ACTION_NAME_KEY.to_owned(),
            Value::Text("process-check-result".to_owned()),
        );
        action.payload.insert(
            ICINGA2_ACTION_PAYLOAD_KEY.to_owned(),
            Value::Map(hashmap![
                "filter".to_owned() => Value::Text("filter_value".to_owned()),
                "type".to_owned() => Value::Text("Host".to_owned())
            ]),
        );

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_ok());
        assert_eq!(
            Some(Icinga2Action {
                name: "process-check-result".to_owned(),
                payload: hashmap![
                    "filter".to_owned() => Value::Text("filter_value".to_owned()),
                    "type".to_owned() => Value::Text("Host".to_owned())
                ]
            }),
            *callback_called.lock().unwrap()
        );
    }
}
