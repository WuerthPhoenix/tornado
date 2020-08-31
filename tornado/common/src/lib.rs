use thiserror::Error;

pub mod actors;
pub mod pool;

#[derive(Error, Debug)]
pub enum TornadoError {
    #[error("ActorCreationError: {message}")]
    ActorCreationError { message: String },
    #[error("ConfigurationError: {message}")]
    ConfigurationError { message: String },
}
