use log::*;
use std::str::FromStr;
use tornado_common_api::Action;
use tornado_executor_common::{Executor, ExecutorError};

pub const ICINGA2_ACTION_KEY: &str = "action";

/// An executor that logs received actions at the 'info' level
#[derive(Default)]
pub struct Icinga2Executor<F: Fn() -> Result<(), ExecutorError>> {
    icinga2_ip: String,
    icinga2_port: u32,
    icinga2_user: String,
    icinga2_pass: String,
    callback: F,
}

impl<F: Fn() -> Result<(), ExecutorError>> Icinga2Executor<F> {
    pub fn new<IP: Into<String>, U: Into<String>, P: Into<String>>(
        icinga2_ip: IP,
        icinga2_port: u32,
        icinga2_user: U,
        icinga2_pass: P,
        callback: F,
    ) -> Icinga2Executor<F> {
        Icinga2Executor {
            icinga2_ip: icinga2_ip.into(),
            icinga2_port,
            icinga2_user: icinga2_user.into(),
            icinga2_pass: icinga2_pass.into(),
            callback,
        }
    }
}

impl<F: Fn() -> Result<(), ExecutorError>> Executor for Icinga2Executor<F> {
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError> {
        debug!("Icinga2Executor - received action: \n[{:#?}]", action);

        match action.payload.get(ICINGA2_ACTION_KEY).and_then(|value| value.get_text()) {
            Some(action) => match Icinga2Action::from_str(action) {
                Ok(icinga2_action) => {
                    info!("Icinga2Executor - perform Icinga2Action: \n[{:#?}]", icinga2_action);
                    (self.callback)()
                }
                Err(_) => Err(ExecutorError::UnknownArgumentError {
                    message: format!("Unknown Icinga2 Action: [{}]", action),
                }),
            },
            None => Err(ExecutorError::MissingArgumentError {
                message: "Icinga2 Action not specified".to_string(),
            }),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Icinga2Action {
    ProcessCheckResult,
}

impl FromStr for Icinga2Action {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "process-check-result" => Ok(Icinga2Action::ProcessCheckResult),
            _ => Err(()),
        }
    }
}

impl Icinga2Action {
    pub fn process() {}
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tornado_common_api::Value;

    #[test]
    fn should_fail_if_action_missing() {
        // Arrange
        let pass = "";
        let callback_called = Arc::new(Mutex::new(false));

        let mut executor = Icinga2Executor::new("127.0.0.1", 5665, "root", pass, || {
            let mut called = callback_called.lock().unwrap();
            *called = true;
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
        assert!(!*callback_called.lock().unwrap());
    }

    #[test]
    fn should_fail_if_action_is_unknown() {
        // Arrange
        let pass = "";
        let callback_called = Arc::new(Mutex::new(false));
        let mut executor = Icinga2Executor::new("127.0.0.1", 5665, "root", pass, || {
            let mut called = callback_called.lock().unwrap();
            *called = true;
            Ok(())
        });

        let mut action = Action::new("");
        action
            .payload
            .insert(ICINGA2_ACTION_KEY.to_owned(), Value::Text("unknown-action-test".to_owned()));

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_err());
        assert_eq!(
            Err(ExecutorError::UnknownArgumentError {
                message: "Unknown Icinga2 Action: [unknown-action-test]".to_owned(),
            }),
            result
        );
        assert!(!*callback_called.lock().unwrap());
    }

    #[test]
    fn should_call_the_callback_if_valid_action() {
        // Arrange
        let pass = "";
        let callback_called = Arc::new(Mutex::new(false));
        let mut executor = Icinga2Executor::new("127.0.0.1", 5665, "root", pass, || {
            let mut called = callback_called.lock().unwrap();
            *called = true;
            Ok(())
        });

        let mut action = Action::new("");
        action
            .payload
            .insert(ICINGA2_ACTION_KEY.to_owned(), Value::Text("process-check-result".to_owned()));

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_ok());
        assert!(*callback_called.lock().unwrap());
    }
}
