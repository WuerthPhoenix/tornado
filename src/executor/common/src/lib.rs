extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate tornado_common_api;

use tornado_common_api::Action;

/// An executor is in charge or performing a specific Action (usually only one, but it could be more).
/// It receives the action description from the Tornado engine and delivers the linked operation.
pub trait Executor {

    /// Executes the operation linked to the received action
    fn execute(&self, action: &Action) -> Result<(), ExecutorError>;
}

#[derive(Fail, Debug)]
pub enum ExecutorError {
    #[fail(display = "ActionExecutionError: [{}]", message)]
    ActionExecutionError { message: String }
}