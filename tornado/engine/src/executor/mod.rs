use actix::prelude::*;
use log::*;
use std::fmt::Display;
use std::sync::Arc;
use tornado_common::pool::Runner;
use tornado_common_api::Action;
use tornado_executor_common::{StatefulExecutor, ExecutorError};

pub mod foreach;
pub mod retry;

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), ExecutorError>")]
pub struct ActionMessage {
    pub action: Arc<Action>,
}

pub struct ExecutorRunner<E: StatefulExecutor + Display + Unpin + Sync + Send> {
    pub executor: E,
}

impl<E: StatefulExecutor + Display + Unpin + Sync + Send + 'static>
    Runner<ActionMessage, Result<(), ExecutorError>> for ExecutorRunner<E>
{
    fn execute(&mut self, msg: ActionMessage) -> Result<(), ExecutorError> {
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
