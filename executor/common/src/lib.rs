use std::sync::Arc;
use thiserror::Error;
use tornado_common_api::{Action, RetriableError};
use std::fmt::Display;
use std::collections::HashMap;
use serde_json::Value;

/// An executor is in charge of performing a specific Action (typically only one, but perhaps more).
/// It receives the Action description from the Tornado engine and delivers the linked operation.
#[async_trait::async_trait(?Send)]
pub trait StatefulExecutor: Display {
    /// Executes the operation linked to the received Action.
    async fn execute(&mut self, action: Arc<Action>) -> Result<(), ExecutorError>;
}

/// An executor is in charge of performing a specific Action (typically only one, but perhaps more).
/// It receives the Action description from the Tornado engine and delivers the linked operation.
#[async_trait::async_trait(?Send)]
pub trait StatelessExecutor: Display {
    /// Executes the operation linked to the received Action.
    async fn execute(&self, action: Arc<Action>) -> Result<(), ExecutorError>;
}

#[derive(Error, Debug, PartialEq)]
pub enum ExecutorError {
    #[error("ActionExecutionError: [{message}], can_retry: {can_retry}, code: {code:?}, data: {data:?}")]
    ActionExecutionError { message: String, can_retry: bool, code: Option<&'static str>, data: HashMap<&'static str, Value> },
    #[error("ConfigurationError: [{message}]")]
    ConfigurationError { message: String },
    #[error("JsonError: {cause}")]
    JsonError { cause: String },
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

impl From<serde_json::Error> for ExecutorError {
    fn from(err: serde_json::Error) -> Self {
        ExecutorError::JsonError { cause: format!("{:?}", err) }
    }
}
