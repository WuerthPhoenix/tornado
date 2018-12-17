extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate tornado_common_api;

use tornado_common_api::Action;

/// An executor is in charge of performing a specific Action (typically only one, but perhaps more).
/// It receives the Action description from the Tornado engine and delivers the linked operation.
pub trait Executor {
    /// Executes the operation linked to the received Action.
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError>;
}

#[derive(Fail, Debug)]
pub enum ExecutorError {
    #[fail(display = "ActionExecutionError: [{}]", message)]
    ActionExecutionError { message: String },
}
