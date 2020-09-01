use actix::prelude::*;
use log::*;
use std::fmt::Display;
use std::sync::Arc;
use tornado_common_api::Action;
use tornado_executor_common::{Executor, ExecutorError};

pub mod foreach;
pub mod retry;

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), ExecutorError>")]
pub struct ActionMessage {
    pub action: Arc<Action>,
}

pub struct ExecutorActor<E: Executor + Display + Unpin> {
    pub executor: E,
}

impl<E: Executor + Display + Unpin + 'static> Actor for ExecutorActor<E> {
    type Context = SyncContext<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("ExecutorActor started.");
    }
}

impl<E: Executor + Display + Unpin + 'static> Handler<ActionMessage> for ExecutorActor<E> {
    type Result = Result<(), ExecutorError>;

    fn handle(&mut self, msg: ActionMessage, _: &mut SyncContext<Self>) -> Self::Result {
        trace!("ExecutorActor - received new action [{:?}]", &msg.action);
        match self.executor.execute(&msg.action) {
            Ok(_) => {
                debug!("ExecutorActor - {} - Action executed successfully", &self.executor);
                Ok(())
            }
            Err(e) => {
                error!("ExecutorActor - {} - Failed to execute action: {}", &self.executor, e);
                Err(e)
            }
        }
    }
}
