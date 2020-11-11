use std::rc::Rc;
use thiserror::Error;
use tornado_common_api::Action;

pub mod callback;
pub mod pool;
pub mod retry;

/// An executor is in charge of performing a specific Action (typically only one, but perhaps more).
/// It receives the Action description from the Tornado engine and delivers the linked operation.
#[async_trait::async_trait(?Send)]
pub trait StatefulExecutor {
    /// Executes the operation linked to the received Action.
    async fn execute(&mut self, action: Rc<Action>) -> Result<(), ExecutorError>;
}

/// An executor is in charge of performing a specific Action (typically only one, but perhaps more).
/// It receives the Action description from the Tornado engine and delivers the linked operation.
#[async_trait::async_trait(?Send)]
pub trait StatelessExecutor {
    /// Executes the operation linked to the received Action.
    async fn execute(&self, action: Rc<Action>) -> Result<(), ExecutorError>;
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
