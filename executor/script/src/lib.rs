use log::*;
use std::fmt;
use std::sync::Arc;
use tokio::process::Command;
use tornado_common_api::{Action, Payload, Value};
use tornado_executor_common::{ExecutorError, StatelessExecutor};
use tracing::instrument;

pub const SCRIPT_TYPE_KEY: &str = "script";
pub const SCRIPT_ARGS_KEY: &str = "args";

#[derive(Default, Clone)]
pub struct ScriptExecutor {}

struct Params<'a> {
    script: String,
    args: Option<&'a Value>,
}

impl ScriptExecutor {
    pub fn new() -> ScriptExecutor {
        Default::default()
    }

    fn append_args(cmd: &mut Command, value: &Value) {
        match value {
            Value::String(args) => {
                cmd.arg(args);
            }
            Value::Bool(arg) => {
                cmd.arg(arg.to_string());
            }
            Value::Number(arg) => {
                cmd.arg(arg.to_string());
            }
            Value::Array(args) => {
                for value in args {
                    ScriptExecutor::append_args(cmd, value);
                }
            }
            Value::Object(args) => {
                for (key, value) in args {
                    cmd.arg(format!("--{}", key));
                    ScriptExecutor::append_args(cmd, value);
                }
            }
            Value::Null => warn!("Args in payload is null. Ignore it."),
        };
    }

    #[instrument(level = "debug", name = "Extract parameters for Executor", skip_all)]
    fn extract_params_from_payload<'a>(
        &self,
        payload: &'a Payload,
    ) -> Result<Params<'a>, ExecutorError> {
        let script = payload
            .get(SCRIPT_TYPE_KEY)
            .and_then(tornado_common_api::ValueExt::get_text)
            .ok_or_else(|| ExecutorError::ActionExecutionError {
                can_retry: false,
                message: format!("Cannot find entry [{}] in the action payload.", SCRIPT_TYPE_KEY),
                code: None,
                data: Default::default(),
            })?
            .to_owned();
        let args = payload.get(SCRIPT_ARGS_KEY);
        Ok(Params { script, args })
    }

    #[instrument(level = "debug", name = "ScriptExecutor", skip_all, fields(otel.name = format!("Execute script: [{}]. Args: {:?}", script, args).as_str()))]
    async fn execute_script(script: String, args: Option<&Value>) -> Result<(), ExecutorError> {
        let output = {
            let script_iter = script.split_whitespace().collect::<Vec<&str>>();
            let mut script_iter = script_iter.iter();
            let mut cmd = Command::new(script_iter.next().ok_or_else(|| {
                ExecutorError::ActionExecutionError {
                    can_retry: false,
                    message: "The script in the payload is empty".to_owned(),
                    code: None,
                    data: Default::default(),
                }
            })?);

            for arg in script_iter {
                cmd.arg(arg);
            }

            if let Some(value) = args {
                ScriptExecutor::append_args(&mut cmd, value);
            } else {
                trace!("No args found in payload")
            };

            cmd.output().await.map_err(|err| ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!("Cannot execute script [{:?}]: {}", script, err),
                code: None,
                data: Default::default(),
            })?
        };

        if output.status.success() {
            debug!(
                "ScriptExecutor - Script completed successfully with status: [{}] - script: [{:?}]",
                output.status, script
            );
            Ok(())
        } else {
            let stderr = String::from_utf8(output.stderr).unwrap_or_default();
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
                data: Default::default(),
            })
        }
    }
}

impl fmt::Display for ScriptExecutor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("ScriptExecutor")
    }
}

#[async_trait::async_trait(?Send)]
impl StatelessExecutor for ScriptExecutor {
    #[tracing::instrument(level = "info", skip_all, err, fields(otel.name = format!("Execute Action: {}", &action.id).as_str(), otel.kind = "Consumer"))]
    async fn execute(&self, action: Arc<Action>) -> Result<(), ExecutorError> {
        trace!("ScriptExecutor - received action: \n{:?}", action);

        let params = self.extract_params_from_payload(&action.payload)?;
        let script = params.script;
        let args = params.args;

        ScriptExecutor::execute_script(script, args).await
    }
}

#[cfg(all(test, unix))]
mod test_unix {

    use super::*;
    use tornado_common_api::{Action, Map, Value};

    #[tokio::test]
    async fn should_return_error_if_script_not_found() {
        // Arrange
        let script = "NOT_EXISTING_SCRIPT.sh";

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::String(script.to_owned()));

        let executor = ScriptExecutor::new();

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        assert!(result.is_err())
    }

    #[tokio::test]
    async fn should_execute_failing_script_and_return_error() {
        // Arrange
        let script = "./test_resources/fail.sh";

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::String(script.to_owned()));

        let executor = ScriptExecutor::new();

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        assert!(result.is_err())
    }

    #[tokio::test]
    async fn should_execute_echo_script() {
        // Arrange
        let script = "./test_resources/echo.sh".to_string();

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::String(script));

        let executor = ScriptExecutor::new();

        // Act
        executor.execute(action.into()).await.unwrap();
    }

    #[tokio::test]
    async fn should_execute_echo_script_with_args() {
        // Arrange
        let script = "./test_resources/echo.sh".to_string();

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::String(script));
        action.payload.insert(SCRIPT_ARGS_KEY.to_owned(), Value::String("hello_world!".to_owned()));

        let executor = ScriptExecutor::new();

        // Act
        executor.execute(action.into()).await.unwrap();
    }

    #[tokio::test]
    async fn should_execute_script_without_arguments() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
        let content = "HelloRustyWorld!";
        let script = format!("{} {} {}", "./test_resources/write_file.sh", &filename, &content);

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::String(script));

        let executor = ScriptExecutor::new();

        // Act
        executor.execute(action.into()).await.unwrap();

        // Assert
        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }

    #[tokio::test]
    async fn should_execute_script_with_arguments() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
        let content = "HelloRustyWorld!";
        let script = format!("{} {}", "./test_resources/write_file.sh", &filename);

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::String(script));
        action.payload.insert(SCRIPT_ARGS_KEY.to_owned(), Value::String(content.to_owned()));

        let executor = ScriptExecutor::new();

        // Act
        executor.execute(action.into()).await.unwrap();

        // Assert
        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }

    #[tokio::test]
    async fn should_execute_script_with_array_of_arguments() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
        let content = "HelloRustyWorld!";
        let script = "./test_resources/write_file.sh".to_owned();

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::String(script));
        action.payload.insert(
            SCRIPT_ARGS_KEY.to_owned(),
            Value::Array(vec![
                Value::String(filename.to_owned()),
                Value::String(content.to_owned()),
            ]),
        );

        let executor = ScriptExecutor::new();

        // Act
        executor.execute(action.into()).await.unwrap();

        // Assert
        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }

    #[tokio::test]
    async fn should_execute_script_escaping_arguments() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());
        let content =
            r#"Hello Rusty World!! 'single quote' "double quote" ""double double quote"""#;
        let script = "./test_resources/write_file.sh".to_owned();

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::String(script));
        action.payload.insert(
            SCRIPT_ARGS_KEY.to_owned(),
            Value::Array(vec![
                Value::String(filename.to_owned()),
                Value::String(content.to_owned()),
            ]),
        );

        let executor = ScriptExecutor::new();

        // Act
        executor.execute(action.into()).await.unwrap();

        // Assert
        let file_content = std::fs::read_to_string(&filename).unwrap();
        assert_eq!(content, file_content.trim())
    }

    #[tokio::test]
    async fn should_execute_script_with_map_of_arguments() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let filename = format!("{}/output.txt", tempdir.path().to_str().unwrap().to_owned());

        let script = "./test_resources/write_all_args_to_file.sh".to_owned();

        let mut action = Action::new("script");
        action.payload.insert(SCRIPT_TYPE_KEY.to_owned(), Value::String(script));

        let first_content = "First_HelloRustyWorld!";
        let second_content = "Second Hello Rusty World!";

        let mut args = Map::new();
        args.insert("first".to_owned(), Value::String(first_content.to_owned()));
        args.insert("second".to_owned(), Value::String(second_content.to_owned()));

        action.payload.insert(
            SCRIPT_ARGS_KEY.to_owned(),
            Value::Array(vec![Value::String(filename.to_owned()), Value::Object(args)]),
        );

        let executor = ScriptExecutor::new();

        // Act
        executor.execute(action.into()).await.unwrap();

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
