use log::*;
use std::fmt;
use std::process::Command;
use tornado_common_api::{Action, Number, Value};
use tornado_executor_common::{Executor, ExecutorError};

pub const SCRIPT_TYPE_KEY: &str = "script";
pub const SCRIPT_ARGS_KEY: &str = "args";

const SHELL: [&str; 2] = ["sh", "-c"];

#[derive(Default)]
pub struct ScriptExecutor {}

impl ScriptExecutor {
    pub fn new() -> ScriptExecutor {
        Default::default()
    }

    fn append_params(script: &mut String, value: &Value) -> Result<(), ExecutorError> {
        match value {
            Value::Text(args) => {
                script.push_str(" ");
                script.push_str(args)
            }
            Value::Bool(arg) => {
                script.push_str(" ");
                script.push_str(&arg.to_string())
            }
            Value::Number(arg) => {
                script.push_str(" ");
                match arg {
                    Number::NegInt(num) => script.push_str(&num.to_string()),
                    Number::PosInt(num) => script.push_str(&num.to_string()),
                    Number::Float(num) => script.push_str(&num.to_string()),
                }
            }
            Value::Array(args) => {
                for value in args {
                    ScriptExecutor::append_params(script, value)?;
                }
            }
            Value::Map(args) => {
                for (key, value) in args {
                    script.push_str(" --");
                    script.push_str(key);
                    ScriptExecutor::append_params(script, value)?;
                }
            }
            Value::Null => warn!("Args in payload is null. Ignore it."),
        };

        Ok(())
    }
}

impl fmt::Display for ScriptExecutor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ScriptExecutor")
    }
}

impl Executor for ScriptExecutor {
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError> {
        trace!("ScriptExecutor - received action: \n{:?}", action);

        let mut script = action
            .payload
            .get(SCRIPT_TYPE_KEY)
            .and_then(tornado_common_api::Value::get_text)
            .ok_or_else(|| ExecutorError::ActionExecutionError {
                message: format!("Cannot find entry [{}] in the action payload.", SCRIPT_TYPE_KEY),
            })?
            .to_owned();

        if let Some(value) = action.payload.get(SCRIPT_ARGS_KEY) {
            ScriptExecutor::append_params(&mut script, &value)?;
        } else {
            trace!("No args found in payload")
        };

        let output =
            Command::new(SHELL[0]).args(&SHELL[1..]).arg(&script).output().map_err(|err| {
                ExecutorError::ActionExecutionError {
                    message: format!("Cannot execute script [{}]: {}", &script, err),
                }
            })?;

        if output.status.success() {
            debug!(
                "ScriptExecutor - Script completed successfully with status: [{}] - script: [{}]",
                &script, output.status
            );
        } else {
            error!(
                "ScriptExecutor - Script returned error status: [{}] - script: [{}]",
                &script, output.status
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;
    use tornado_common_api::Value;

    #[test]
    fn should_append_text_placeholders() {
        // Arrange
        let mut script = "./script.sh".to_owned();

        let first_content = "First_HelloRustyWorld!";
        let second_content = "Second_HelloRustyWorld!";

        let expected_final_script = "./script.sh First_HelloRustyWorld! Second_HelloRustyWorld!";

        let args = Value::Text(format!("{} {}", first_content, second_content));

        // Act
        ScriptExecutor::append_params(&mut script, &args).unwrap();

        // Assert
        assert_eq!(expected_final_script, script);
    }

    #[test]
    fn should_append_array_placeholders() {
        // Arrange
        let mut script = "./script.sh".to_owned();

        let first_content = "First_HelloRustyWorld!";
        let second_content = "Second_HelloRustyWorld!";

        let expected_final_script =
            "./script.sh First_HelloRustyWorld! Second_HelloRustyWorld! true 1123";

        let args = Value::Array(vec![
            Value::Text(first_content.to_owned()),
            Value::Text(second_content.to_owned()),
            Value::Bool(true),
            Value::Number(Number::PosInt(1123)),
        ]);

        // Act
        ScriptExecutor::append_params(&mut script, &args).unwrap();

        // Assert
        assert_eq!(expected_final_script, script);
    }

    #[test]
    fn should_append_map_placeholders() {
        // Arrange
        let mut script = "./script.sh".to_owned();

        let first_content = "First_HelloRustyWorld!";
        let second_content = "Second_HelloRustyWorld!";

        let mut args = HashMap::new();
        args.insert("first".to_owned(), Value::Text(first_content.to_owned()));
        args.insert("second".to_owned(), Value::Text(second_content.to_owned()));

        let mut expected_final_script = "./script.sh".to_owned();

        args.iter().for_each(|(key, value)| {
            expected_final_script.push_str(" --");
            expected_final_script.push_str(key);
            expected_final_script.push_str(" ");
            expected_final_script.push_str(value.get_text().unwrap());
        });

        let args = Value::Map(args);

        // Act
        ScriptExecutor::append_params(&mut script, &args).unwrap();

        // Assert
        assert_eq!(expected_final_script, script);
        assert!(script.contains(" --first First_HelloRustyWorld!"));
        assert!(script.contains(" --second Second_HelloRustyWorld!"));
    }

    #[test]
    fn should_ignore_null_placeholders() {
        // Arrange
        let mut script = "./script.sh".to_owned();

        let first_content = "--something good";

        let expected_final_script = "./script.sh --something good";

        let args = Value::Array(vec![
            Value::Null,
            Value::Text(first_content.to_owned()),
            Value::Null,
            Value::Null,
        ]);

        // Act
        ScriptExecutor::append_params(&mut script, &args).unwrap();

        // Assert
        assert_eq!(expected_final_script, script);
    }
}

#[cfg(all(test, unix))]
mod test_unix {

    use super::*;
    use std::process::Command;
    use tornado_common_api::Value;

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
    fn should_execute_script_without_placeholders() {
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
    fn should_execute_script_with_placeholders() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
        let content = "HelloRustyWorld!";
        let script = format!("{} {}", "./test_resources/write_file.sh", &filename);

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::Text(script));
        action.payload.insert(SCRIPT_ARGS_KEY.to_owned(), Value::Text(content.to_owned()));

        let mut executor = ScriptExecutor::new();

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_ok());

        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }
}
