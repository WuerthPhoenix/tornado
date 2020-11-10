use crate::{ExecutorError, StatefulExecutor, StatelessExecutor};
use std::future::Future;
use std::rc::Rc;
use tornado_common_api::Action;

pub struct CallbackStatefulExecutor<
    F: Fn(Rc<Action>) -> Fut,
    Fut: Future<Output = Result<(), ExecutorError>>,
> {
    callback: F,
}

impl<F: Fn(Rc<Action>) -> Fut, Fut: Future<Output = Result<(), ExecutorError>>>
    CallbackStatefulExecutor<F, Fut>
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

#[async_trait::async_trait(?Send)]
impl<F: Fn(Rc<Action>) -> Fut, Fut: Future<Output = Result<(), ExecutorError>>> StatefulExecutor
    for CallbackStatefulExecutor<F, Fut>
{
    async fn execute(&mut self, action: Rc<Action>) -> Result<(), ExecutorError> {
        (self.callback)(action).await
    }
}

pub struct CallbackStatelessExecutor<
    F: Fn(Rc<Action>) -> Fut,
    Fut: Future<Output = Result<(), ExecutorError>>,
> {
    callback: F,
}

impl<F: Fn(Rc<Action>) -> Fut, Fut: Future<Output = Result<(), ExecutorError>>>
    CallbackStatelessExecutor<F, Fut>
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

#[async_trait::async_trait(?Send)]
impl<F: Fn(Rc<Action>) -> Fut, Fut: Future<Output = Result<(), ExecutorError>>> StatelessExecutor
    for CallbackStatelessExecutor<F, Fut>
{
    async fn execute(&self, action: Rc<Action>) -> Result<(), ExecutorError> {
        (self.callback)(action).await
    }
}
