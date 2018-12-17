#[macro_use]
extern crate log;
extern crate regex;
extern crate tornado_common_api;
extern crate tornado_executor_common;

use regex::Regex;
use std::fmt;
use std::process::Command;
use tornado_common_api::{Action, Payload};
use tornado_executor_common::{Executor, ExecutorError};

pub const SCRIPT_TYPE_KEY: &str = "script";
const PARAMS_REGEX: &str = r"\$\{[^\}]+\}";
const SHELL: [&str; 2] = ["sh", "-c"];

pub struct ScriptExecutor {
    params_regex: Regex,
}

impl Default for ScriptExecutor {
    fn default() -> Self {
        ScriptExecutor {
            params_regex: Regex::new(PARAMS_REGEX).expect("ScriptExecutor regex should be valid"),
        }
    }
}

impl ScriptExecutor {
    pub fn new() -> ScriptExecutor {
        Default::default()
    }

    fn replace_params(&self, script: &str, payload: &Payload) -> Result<String, ExecutorError> {
        let mut script_final = script.to_owned();
        for capture in self.params_regex.captures_iter(script) {
            if let Some(value) = capture.get(0) {
                let group = value.as_str();
                let param = &group[2..group.len() - 1];

                let param_value =
                    payload.get(param).and_then(|value| value.text()).ok_or_else(|| {
                        ExecutorError::ActionExecutionError {
                            message: format!(
                                "Cannot find entry [{}] in the action payload for script [{}].",
                                &param, script
                            ),
                        }
                    })?;

                script_final = script_final.replace(group, param_value);
            }
        }
        Ok(script_final)
    }
}

impl fmt::Display for ScriptExecutor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ScriptExecutor")
    }
}

impl Executor for ScriptExecutor {
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError> {
        debug!("ScriptExecutor - received action: \n{:#?}", action);

        let script = action
            .payload
            .get(SCRIPT_TYPE_KEY)
            .and_then(|value| value.text())
            .ok_or_else(|| ExecutorError::ActionExecutionError {
                message: format!("Cannot find entry [{}] in the action payload.", SCRIPT_TYPE_KEY),
            })?;

        let final_script = self.replace_params(script, &action.payload)?;

        let output = Command::new(SHELL[0]).args(&SHELL[1..]).arg(&final_script).output().map_err(
            |err| ExecutorError::ActionExecutionError {
                message: format!("Cannot execute script [{}]: {}", &final_script, err),
            },
        )?;

        debug!(
            "ScriptExecutor - executed: [{}] - Status: {}",
            &final_script,
            output.status
        );

        Ok(())
    }
}

#[cfg(test)]
extern crate tempfile;

#[cfg(test)]
mod test {

    use super::*;
    use std::process::Command;
    use tornado_common_api::{Payload, Value};

    #[test]
    fn spike_command_script() {
        let output = Command::new("./test_resources/echo.sh")
            .arg("hello_world")
            .output()
            .expect("failed to execute process");

        println!("status: {}", output.status);
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout).trim());
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr).trim());

        assert_eq!("hello_world", String::from_utf8_lossy(&output.stdout).trim());
        assert!(output.status.success());
    }

    #[test]
    fn spike_command_failing_script() {
        let output =
            Command::new("./test_resources/fail.sh").output().expect("failed to execute process");

        println!("status: {}", output.status);

        assert!(!output.status.success());
    }

    #[test]
    fn spike_command_script_with_inline_args() {
        let shell: [&str; 2] = ["sh", "-c"];
        let output = Command::new(shell[0])
            .args(&shell[1..])
            .arg("./test_resources/echo.sh hello_world")
            .output()
            .expect("failed to execute process");

        println!("status: {}", output.status);
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout).trim());
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr).trim());

        assert_eq!("hello_world", String::from_utf8_lossy(&output.stdout).trim());
        assert!(output.status.success());
    }

    #[test]
    fn spike_execute_script_write_file() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
        let content = "HelloRustyWorld!";

        // Act
        let output = Command::new("./test_resources/write_file.sh")
            .arg(&filename)
            .arg(&content)
            .output()
            .expect("failed to execute process");

        // Assert
        assert!(output.status.success());

        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }

    #[test]
    fn should_execute_script_write_file() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
        let content = "HelloRustyWorld!";
        let script = format!("{} {} {}", "./test_resources/write_file.sh", &filename, &content);

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::Text(script));

        let mut executor = ScriptExecutor::new();

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_ok());

        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }

    #[test]
    fn should_replace_placeholders() {
        // Arrange
        let script = format!("{} {} {}", "./script.sh", "${first}", "${second}");

        let first_content = "First_HelloRustyWorld!";
        let second_content = "Second_HelloRustyWorld!";

        let expected_final_script =
            format!("{} {} {}", "./script.sh", first_content, second_content);

        let mut payload = Payload::new();
        payload.insert("first".to_owned(), Value::Text(first_content.to_owned()));
        payload.insert("second".to_owned(), Value::Text(second_content.to_owned()));

        let executor = ScriptExecutor::new();

        // Act
        let result = executor.replace_params(&script, &payload).unwrap();

        // Assert
        assert_eq!(expected_final_script, result);
    }

    #[test]
    fn replace_placeholders_should_return_original_script_if_no_params() {
        // Arrange
        let script = format!("{} {} {}", "./script.sh", "first", "second");

        let payload = Payload::new();

        let executor = ScriptExecutor::new();

        // Act
        let result = executor.replace_params(&script, &payload).unwrap();

        // Assert
        assert_eq!(script, result);
    }

    #[test]
    fn replace_placeholders_should_fail_if_missing_params() {
        // Arrange
        let script = format!("{} {} {}", "./script.sh", "${first}", "${second}");

        let mut payload = Payload::new();
        payload.insert("first".to_owned(), Value::Text("value".to_owned()));

        let executor = ScriptExecutor::new();

        // Act
        let result = executor.replace_params(&script, &payload);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_execute_script_with_placeholders() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
        let content = "HelloRustyWorld!";
        let script = format!("{} {} {}", "./test_resources/write_file.sh", &filename, "${content}");

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::Text(script));
        action.payload.insert("content".to_owned(), Value::Text(content.to_owned()));

        let mut executor = ScriptExecutor::new();

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_ok());

        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }

}
