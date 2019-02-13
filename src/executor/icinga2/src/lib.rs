use log::*;
use std::collections::HashMap;
use tornado_common_api::{Action, Value};
use tornado_executor_common::{Executor, ExecutorError};

pub const ICINGA2_ACTION_KEY: &str = "action";

/// An executor that logs received actions at the 'info' level
#[derive(Default)]
pub struct Icinga2Executor {
    icinga2_ip: String,
    icinga2_port: u32,
    icinga2_user: String,
    icinga2_pass: String,
}

impl Icinga2Executor {
    pub fn new<IP: Into<String>, U: Into<String>, P: Into<String>>(
        icinga2_ip: IP,
        icinga2_port: u32,
        icinga2_user: U,
        icinga2_pass: P,
    ) -> Icinga2Executor {
        Icinga2Executor {
            icinga2_ip: icinga2_ip.into(),
            icinga2_port,
            icinga2_user: icinga2_user.into(),
            icinga2_pass: icinga2_pass.into(),
        }
    }
}

impl Executor for Icinga2Executor {
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError> {
        debug!("Icinga2Executor - received action: \n[{:#?}]", action);

        match action.payload.get(ICINGA2_ACTION_KEY).and_then(|value| value.get_text()) {
            Some(action) => match Icinga2Action::from_str(action) {
                Some(icinga2_action) => {
                    info!("Icinga2Executor - perform Icinga2Action: \n[{:#?}]", icinga2_action);
                    send_request();
                    Ok(())
                }
                None => Err(ExecutorError::UnknownArgumentError {
                    message: format!("Unknown Icinga2 Action: [{}]", action),
                }),
            },
            None => Err(ExecutorError::MissingArgumentError {
                message: format!("Icinga2 Action not specified"),
            }),
        }
    }
}

// ToDo: temporary solution while evaluating how to structure the code.
fn send_request() {
    println!("Send request")
}

#[derive(Debug, PartialEq)]
pub enum Icinga2Action {
    ProcessCheckResult,
}

impl Icinga2Action {
    pub fn from_str(s: &str) -> Option<Icinga2Action> {
        match s {
            "ProcessCheckResult" => Some(Icinga2Action::ProcessCheckResult),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_fail_if_action_missing() {
        // Arrange
        let pass = "";
        let mut executor = Icinga2Executor::new("127.0.0.1", 5665, "root", pass);

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
    }

    #[test]
    fn should_fail_if_action_is_unknown() {
        // Arrange
        let pass = "";
        let mut executor = Icinga2Executor::new("127.0.0.1", 5665, "root", pass);

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
    }
}
