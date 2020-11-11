use thiserror::Error;

pub mod actors;
pub mod command;
pub mod pool;

#[derive(Error, Debug)]
pub enum TornadoError {
    #[error("SenderError: {message}")]
    SenderError { message: String },
    #[error("ActorCreationError: {message}")]
    ActorCreationError { message: String },
    #[error("ConfigurationError: {message}")]
    ConfigurationError { message: String },
    #[error("ExecutionError: {message}")]
    ExecutionError { message: String },
}
