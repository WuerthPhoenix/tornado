use log::*;
use std::fmt;
use std::process::Command;
use tornado_common_api::{Action, Number, Value};
use tornado_executor_common::{Executor, ExecutorError};

pub const SCRIPT_TYPE_KEY: &str = "script";
pub const SCRIPT_ARGS_KEY: &str = "args";

#[derive(Default)]
pub struct ScriptExecutor {}

impl ScriptExecutor {
    pub fn new() -> ScriptExecutor {
        Default::default()
    }

    fn append_args(cmd: &mut Command, value: &Value) {
        match value {
            Value::Text(args) => {
                cmd.arg(args);
            }
            Value::Bool(arg) => {
                cmd.arg(&arg.to_string());
            }
            Value::Number(arg) => {
                match arg {
                    Number::NegInt(num) => cmd.arg(&num.to_string()),
                    Number::PosInt(num) => cmd.arg(&num.to_string()),
                    Number::Float(num) => cmd.arg(&num.to_string()),
                };
            }
            Value::Array(args) => {
                for value in args {
                    ScriptExecutor::append_args(cmd, value);
                }
            }
            Value::Map(args) => {
                for (key, value) in args {
                    cmd.arg(&format!("--{}", key));
                    ScriptExecutor::append_args(cmd, value);
                }
            }
            Value::Null => warn!("Args in payload is null. Ignore it."),
        };
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

        let script = action
            .payload
            .get(SCRIPT_TYPE_KEY)
            .and_then(tornado_common_api::Value::get_text)
            .ok_or_else(|| ExecutorError::ActionExecutionError {
                can_retry: false,
                message: format!("Cannot find entry [{}] in the action payload.", SCRIPT_TYPE_KEY),
                code: None,
            })?
            .to_owned();

        let output = {
            let script_iter = script.split_whitespace().collect::<Vec<&str>>();
            let mut script_iter = script_iter.iter();
            let mut cmd = Command::new(script_iter.next().ok_or_else(|| {
                ExecutorError::ActionExecutionError {
                    can_retry: false,
                    message: "The script in the payload is empty".to_owned(),
                    code: None,
                }
            })?);

            for arg in script_iter {
                cmd.arg(arg);
            }

            if let Some(value) = action.payload.get(SCRIPT_ARGS_KEY) {
                ScriptExecutor::append_args(&mut cmd, value);
            } else {
                trace!("No args found in payload")
            };

            cmd.output().map_err(|err| ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!("Cannot execute script [{:?}]: {}", script, err),
                code: None,
            })?
        };

        if output.status.success() {
            debug!(
                "ScriptExecutor - Script completed successfully with status: [{}] - script: [{:?}]",
                output.status, script
            );
            Ok(())
        } else {
            let stderr = String::from_utf8(output.stdout).unwrap_or_default();
            error!(
                "ScriptExecutor - Script returned error status: [{}] - script: [{:?}] - stderr: [{}]",
                output.status, script, stderr
            );

            Err(ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!(
                    "Script execution failed with status: [{}] - script: [{:?}] - stderr: [{}]",
                    output.status, script, stderr
                ),
                code: None,
            })
        }
    }
}

#[cfg(all(test, unix))]
mod test_unix {

    use super::*;
    use std::collections::HashMap;
    use tornado_common_api::Value;

    #[test]
    fn should_return_error_if_script_not_found() {
        // Arrange
        let script = "NOT_EXISTING_SCRIPT.sh";

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::Text(script.to_owned()));

        let mut executor = ScriptExecutor::new();

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_err())
    }

    #[test]
    fn should_execute_failing_script_and_return_error() {
        // Arrange
        let script = "./test_resources/fail.sh";

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::Text(script.to_owned()));

        let mut executor = ScriptExecutor::new();

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_err())
    }

    #[test]
    fn should_execute_echo_script() {
        // Arrange
        let script = format!("{}", "./test_resources/echo.sh");

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::Text(script));

        let mut executor = ScriptExecutor::new();

        // Act
        executor.execute(&action).unwrap();
    }

    #[test]
    fn should_execute_echo_script_with_args() {
        // Arrange
        let script = format!("{}", "./test_resources/echo.sh");

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::Text(script));
        action.payload.insert(SCRIPT_ARGS_KEY.to_owned(), Value::Text("hello_world!".to_owned()));

        let mut executor = ScriptExecutor::new();

        // Act
        executor.execute(&action).unwrap();
    }

    #[test]
    fn should_execute_script_without_arguments() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
        let content = "HelloRustyWorld!";
        let script = format!("{} {} {}", "./test_resources/write_file.sh", &filename, &content);

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::Text(script));

        let mut executor = ScriptExecutor::new();

        // Act
        executor.execute(&action).unwrap();

        // Assert
        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }

    #[test]
    fn should_execute_script_with_arguments() {
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
        executor.execute(&action).unwrap();

        // Assert
        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }

    #[test]
    fn should_execute_script_with_array_of_arguments() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
        let content = "HelloRustyWorld!";
        let script = "./test_resources/write_file.sh".to_owned();

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::Text(script));
        action.payload.insert(
            SCRIPT_ARGS_KEY.to_owned(),
            Value::Array(vec![Value::Text(filename.to_owned()), Value::Text(content.to_owned())]),
        );

        let mut executor = ScriptExecutor::new();

        // Act
        executor.execute(&action).unwrap();

        // Assert
        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }

    #[test]
    fn should_execute_script_escaping_arguments() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
        let content =
            r#"Hello Rusty World!! 'single quote' "double quote" ""double double quote"""#;
        let script = "./test_resources/write_file.sh".to_owned();

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::Text(script));
        action.payload.insert(
            SCRIPT_ARGS_KEY.to_owned(),
            Value::Array(vec![Value::Text(filename.to_owned()), Value::Text(content.to_owned())]),
        );

        let mut executor = ScriptExecutor::new();

        // Act
        executor.execute(&action).unwrap();

        // Assert
        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }

    #[test]
    fn should_execute_script_with_map_of_arguments() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());

        let script = "./test_resources/write_all_args_to_file.sh".to_owned();

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::Text(script));

        let first_content = "First_HelloRustyWorld!";
        let second_content = "Second Hello Rusty World!";

        let mut args = HashMap::new();
        args.insert("first".to_owned(), Value::Text(first_content.to_owned()));
        args.insert("second".to_owned(), Value::Text(second_content.to_owned()));

        action.payload.insert(
            SCRIPT_ARGS_KEY.to_owned(),
            Value::Array(vec![Value::Text(filename.to_owned()), Value::Map(args)]),
        );

        let mut executor = ScriptExecutor::new();

        // Act
        executor.execute(&action).unwrap();

        // Assert
        let file_content = std::fs::read_to_string(&filename).unwrap();

        // Arguments from a map are not ordered
        let expected_1 = format!(
            r#"
{}
--first
{}
--second
{}"#,
            filename, first_content, second_content
        )
        .trim()
        .to_owned();
        let expected_2 = format!(
            r#"
{}
--second
{}
--first
{}"#,
            filename, second_content, first_content
        )
        .trim()
        .to_owned();

        println!("File content is : [{}]", file_content.trim());
        assert!(file_content.trim().eq(&expected_1) || file_content.trim().eq(&expected_2))
    }
}
