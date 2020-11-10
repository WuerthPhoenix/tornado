use std::rc::Rc;
use tornado_common_api::Action;
use crate::{ExecutorError, StatefulExecutor, StatelessExecutor};

pub struct CallbackStatefulExecutor<F: FnMut(Rc<Action>) -> Result<(), ExecutorError>>{
    callback: F
}

impl <F: FnMut(Rc<Action>) -> Result<(), ExecutorError>> CallbackStatefulExecutor<F> {
    pub fn new(callback: F) -> Self {
        Self {
            callback
        }
    }
}

#[async_trait::async_trait(?Send)]
impl <F: FnMut(Rc<Action>) -> Result<(), ExecutorError>> StatefulExecutor for CallbackStatefulExecutor<F> {

    async fn execute(&mut self, action: Rc<Action>) -> Result<(), ExecutorError> {
        unimplemented!()
    }
}

pub struct CallbackStatelessExecutor<F: Fn(Rc<Action>) -> Result<(), ExecutorError>>{
    callback: F
}

impl <F: Fn(Rc<Action>) -> Result<(), ExecutorError>> CallbackStatelessExecutor<F> {
    pub fn new(callback: F) -> Self {
        Self {
            callback
        }
    }
}

#[async_trait::async_trait(?Send)]
impl <F: Fn(Rc<Action>) -> Result<(), ExecutorError>> StatelessExecutor for CallbackStatelessExecutor<F> {

    async fn execute(&self, action: Rc<Action>) -> Result<(), ExecutorError> {
        unimplemented!()
    }
}