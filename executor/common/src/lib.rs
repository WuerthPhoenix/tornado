use std::sync::Arc;
use thiserror::Error;
use tornado_common_api::{Action, RetriableError};

/// An executor is in charge of performing a specific Action (typically only one, but perhaps more).
/// It receives the Action description from the Tornado engine and delivers the linked operation.
#[async_trait::async_trait(?Send)]
pub trait StatefulExecutor {
    /// Executes the operation linked to the received Action.
    async fn execute(&mut self, action: Arc<Action>) -> Result<(), ExecutorError>;
}

/// An executor is in charge of performing a specific Action (typically only one, but perhaps more).
/// It receives the Action description from the Tornado engine and delivers the linked operation.
#[async_trait::async_trait(?Send)]
pub trait StatelessExecutor {
    /// Executes the operation linked to the received Action.
    async fn execute(&self, action: Arc<Action>) -> Result<(), ExecutorError>;
}

#[derive(Error, Debug, PartialEq, Clone)]
pub enum ExecutorError {
    #[error("ActionExecutionError: [{message}], can_retry: {can_retry}, code: {code:?}")]
    ActionExecutionError { message: String, can_retry: bool, code: Option<&'static str> },
    #[error("ConfigurationError: [{message}]")]
    ConfigurationError { message: String },
    #[error("MissingArgumentError: [{message}]")]
    MissingArgumentError { message: String },
    #[error("SenderError: {message}")]
    SenderError { message: String },
    #[error("UnknownArgumentError: [{message}]")]
    UnknownArgumentError { message: String },
}

impl RetriableError for ExecutorError {
    fn can_retry(&self) -> bool {
        match self {
            ExecutorError::ActionExecutionError { can_retry, .. } => *can_retry,
            _ => false,
        }
    }
}
