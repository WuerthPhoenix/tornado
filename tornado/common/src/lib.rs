use failure_derive::Fail;

pub mod actors;

#[derive(Fail, Debug)]
pub enum TornadoError {
    #[fail(display = "ActorCreationError: {}", message)]
    ActorCreationError { message: String },
    #[fail(display = "ConfigurationError: {}", message)]
    ConfigurationError { message: String },
}
