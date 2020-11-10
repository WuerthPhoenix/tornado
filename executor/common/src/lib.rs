use thiserror::Error;
use tornado_common_api::Action;

/// An executor is in charge of performing a specific Action (typically only one, but perhaps more).
/// It receives the Action description from the Tornado engine and delivers the linked operation.
#[async_trait::async_trait(?Send)]
pub trait Executor {
    /// Executes the operation linked to the received Action.
    async fn execute(&mut self, action: &Action) -> Result<(), ExecutorError>;
}

#[derive(Error, Debug, PartialEq, Clone)]
pub enum ExecutorError {
    #[error("ActionExecutionError: [{message}], can_retry: {can_retry}, code: {code:?}")]
    ActionExecutionError { message: String, can_retry: bool, code: Option<&'static str> },
    #[error("MissingArgumentError: [{message}]")]
    MissingArgumentError { message: String },
    #[error("UnknownArgumentError: [{message}]")]
    UnknownArgumentError { message: String },
    #[error("ConfigurationError: [{message}]")]
    ConfigurationError { message: String },
}

pub trait RetriableError {
    fn can_retry(&self) -> bool;
}

impl RetriableError for ExecutorError {
    fn can_retry(&self) -> bool {
        match self {
            ExecutorError::ActionExecutionError { can_retry, .. } => *can_retry,
            _ => false,
        }
    }
}
