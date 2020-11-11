use std::rc::Rc;
use tornado_common_api::Action;
use tornado_executor_common::{ExecutorError, StatefulExecutor, StatelessExecutor};

pub mod callback;
pub mod pool;
pub mod retry;
pub mod spawn;

/// Basic Trait to implement the Command Design Pattern.
/// See: https://refactoring.guru/design-patterns/command
#[async_trait::async_trait(?Send)]
pub trait Command<Message, Output> {
    async fn execute(&self, message: Message) -> Output;
}

/// Implement the Command pattern for StatelessExecutor
#[async_trait::async_trait(?Send)]
impl<T: StatelessExecutor> Command<Rc<Action>, Result<(), ExecutorError>> for T {
    async fn execute(&self, message: Rc<Action>) -> Result<(), ExecutorError> {
        (self as &T).execute(message).await
    }
}

/// Basic Trait to implement the Command Design Pattern.
/// See: https://refactoring.guru/design-patterns/command
#[async_trait::async_trait(?Send)]
pub trait CommandMut<Message, Output> {
    async fn execute(&mut self, message: Message) -> Output;
}

/// Implement the Command pattern for StatefulExecutor
#[async_trait::async_trait(?Send)]
impl<T: StatefulExecutor> CommandMut<Rc<Action>, Result<(), ExecutorError>> for T {
    async fn execute(&mut self, message: Rc<Action>) -> Result<(), ExecutorError> {
        (self as &mut T).execute(message).await
    }
}
