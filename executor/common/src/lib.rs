use thiserror::Error;
use tornado_common_api::Action;

/// An executor is in charge of performing a specific Action (typically only one, but perhaps more).
/// It receives the Action description from the Tornado engine and delivers the linked operation.
pub trait Executor {
    /// Executes the operation linked to the received Action.
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError>;
}

#[derive(Error, Debug, PartialEq)]
pub enum ExecutorError {
    #[error("ActionExecutionError: [{message}]")]
    ActionExecutionError { message: String },
    #[error("MissingArgumentError: [{message}]")]
    MissingArgumentError { message: String },
    #[error("UnknownArgumentError: [{message}]")]
    UnknownArgumentError { message: String },
    #[error("ConfigurationError: [{message}]")]
    ConfigurationError { message: String },
    #[error("IcingaObjectNotFoundError: [{message}]")]
    IcingaObjectNotFoundError { message: String },
    #[error("IcingaObjectAlreadyExistingError: [{message}]")]
    IcingaObjectAlreadyExistingError { message: String },
}
