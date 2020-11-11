use std::rc::Rc;
use tornado_common_api::Action;
use tornado_executor_common::{ExecutorError, StatefulExecutor, StatelessExecutor};

pub mod callback;
pub mod pool;
pub mod retry;

#[async_trait::async_trait(?Send)]
pub trait Wrapper<Message, Output> {
    async fn execute(&self, message: Message) -> Output;
}

#[async_trait::async_trait(?Send)]
impl<T: StatelessExecutor> Wrapper<Rc<Action>, Result<(), ExecutorError>> for T {
    async fn execute(&self, message: Rc<Action>) -> Result<(), ExecutorError> {
        (self as &T).execute(message).await
    }
}

#[async_trait::async_trait(?Send)]
pub trait WrapperMut<Message, Output> {
    async fn execute(&mut self, message: Message) -> Output;
}

#[async_trait::async_trait(?Send)]
impl<T: StatefulExecutor> WrapperMut<Rc<Action>, Result<(), ExecutorError>> for T {
    async fn execute(&mut self, message: Rc<Action>) -> Result<(), ExecutorError> {
        (self as &mut T).execute(message).await
    }
}
