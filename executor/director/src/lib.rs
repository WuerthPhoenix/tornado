use log::*;
use serde::*;
use tornado_common_api::Action;
use tornado_common_api::Payload;
use tornado_common_api::Value;
use tornado_executor_common::{Executor, ExecutorError};

pub const DIRECTOR_ACTION_NAME_KEY: &str = "action_name";
pub const DIRECTOR_ACTION_PAYLOAD_KEY: &str = "action_payload";
pub const DIRECTOR_ACTION_LIVE_CREATION_KEY: &str = "icinga2_live_creation";

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum DirectorActionName {
    CreateHost,
    CreateService,
}

impl DirectorActionName {
    fn from_str(name: &str) -> Result<Self, ExecutorError> {
        match name {
            "create_host" => Ok(DirectorActionName::CreateHost),
            "create_service" => Ok(DirectorActionName::CreateService),
            val => Err(ExecutorError::UnknownArgumentError { message: format!("Invalid action_name value. Found: '{}'. Expected valid action_name. Refer to the documentation",val) })
        }
    }

    pub fn to_director_api_subpath(&self) -> &str {
        match self {
            DirectorActionName::CreateHost => "host",
            DirectorActionName::CreateService => "service",
        }
    }
}

/// An executor that calls the APIs of the IcingaWeb2 Director
#[derive(Default)]
pub struct DirectorExecutor<F: Fn(DirectorAction) -> Result<(), ExecutorError>> {
    callback: F,
}

impl<F: Fn(DirectorAction) -> Result<(), ExecutorError>> std::fmt::Display for DirectorExecutor<F> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("DirectorExecutor")?;
        Ok(())
    }
}

impl<F: Fn(DirectorAction) -> Result<(), ExecutorError>> DirectorExecutor<F> {
    pub fn new(callback: F) -> DirectorExecutor<F> {
        DirectorExecutor { callback }
    }

    fn get_payload(&self, payload: &mut Payload) -> Result<Value, ExecutorError> {
        payload.remove(DIRECTOR_ACTION_PAYLOAD_KEY).ok_or(ExecutorError::MissingArgumentError {
            message: "Director Action Payload not specified".to_string(),
        })
    }

    fn get_live_creation_setting(&self, payload: &Payload) -> bool {
        payload
            .get(DIRECTOR_ACTION_LIVE_CREATION_KEY)
            .and_then(tornado_common_api::Value::get_bool)
            .unwrap_or(&false)
            .to_owned()
    }
}

impl<F: Fn(DirectorAction) -> Result<(), ExecutorError>> Executor for DirectorExecutor<F> {
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError> {
        trace!("DirectorExecutor - received action: \n[{:?}]", action);

        // ToDo: clone to be removed in TOR-226
        let mut action = action.clone();
        let director_action_name = action
            .payload
            .get(DIRECTOR_ACTION_NAME_KEY)
            .and_then(tornado_common_api::Value::get_text)
            .ok_or(ExecutorError::MissingArgumentError {
                message: "Director Action not specified".to_string(),
            })
            .and_then(DirectorActionName::from_str)?;

        trace!("DirectorExecutor - perform DirectorAction: \n[{:?}]", director_action_name);

        let action_payload = self.get_payload(&mut action.payload)?;

        let live_creation = self.get_live_creation_setting(&action.payload);

        (self.callback)(DirectorAction {
            name: director_action_name,
            payload: action_payload,
            live_creation: live_creation.to_owned(),
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectorAction {
    pub name: DirectorActionName,
    pub payload: Value,
    pub live_creation: bool,
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

        let mut executor = DirectorExecutor::new(|director_action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(director_action);
            Ok(())
        });

        let action = Action::new("");

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_err());
        assert_eq!(
            Err(ExecutorError::MissingArgumentError {
                message: "Director Action not specified".to_owned()
            }),
            result
        );
        assert_eq!(None, *callback_called.lock().unwrap());
    }

    #[test]
    fn should_throw_error_if_action_payload_is_not_set() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));
        let mut executor = DirectorExecutor::new(|director_action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(director_action);
            Ok(())
        });

        let mut action = Action::new("");
        action
            .payload
            .insert(DIRECTOR_ACTION_NAME_KEY.to_owned(), Value::Text("create_service".to_owned()));
        action.payload.insert(DIRECTOR_ACTION_LIVE_CREATION_KEY.to_owned(), Value::Bool(true));

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_call_the_callback_if_valid_action() {
        // Arrange
        let callback_called = Arc::new(Mutex::new(None));
        let mut executor = DirectorExecutor::new(|director_action| {
            let mut called = callback_called.lock().unwrap();
            *called = Some(director_action);
            Ok(())
        });

        let mut action = Action::new("");
        action
            .payload
            .insert(DIRECTOR_ACTION_NAME_KEY.to_owned(), Value::Text("create_host".to_owned()));
        action.payload.insert(
            DIRECTOR_ACTION_PAYLOAD_KEY.to_owned(),
            Value::Map(hashmap![
                "filter".to_owned() => Value::Text("filter_value".to_owned()),
                "type".to_owned() => Value::Text("Host".to_owned())
            ]),
        );

        // Act
        let result = executor.execute(&action);

        println!("{:?}", result);
        // Assert
        assert!(result.is_ok());
        assert_eq!(
            Some(DirectorAction {
                name: DirectorActionName::CreateHost,
                payload: Value::Map(hashmap![
                    "filter".to_owned() => Value::Text("filter_value".to_owned()),
                    "type".to_owned() => Value::Text("Host".to_owned())
                ]),
                live_creation: false
            }),
            *callback_called.lock().unwrap()
        );
    }
}
