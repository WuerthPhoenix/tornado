use actix::prelude::*;
use log::*;
use std::fmt::Display;
use std::sync::Arc;
use tornado_common::pool::blocking_pool::start_blocking_runner;
use tornado_common::pool::Sender;
use tornado_common::TornadoError;
use tornado_common_api::Action;
use tornado_executor_common::{Executor, ExecutorError};

pub mod foreach;
pub mod retry;

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), ExecutorError>")]
pub struct ActionMessage {
    pub action: Arc<Action>,
}

pub struct ExecutorRunner<E: Executor + Display + Unpin + Sync + Send> {
    pub executor: E,
}

impl<E: Executor + Display + Unpin + Sync + Send + 'static> ExecutorRunner<E> {
    pub fn start_new(
        max_parallel_executions: usize,
        buffer_size: usize,
        executor: E,
    ) -> Result<Sender<ActionMessage, Result<(), ExecutorError>>, TornadoError> {
        let executor_runner = Arc::new(ExecutorRunner { executor });

        start_blocking_runner(
            max_parallel_executions,
            buffer_size,
            Arc::new(move |message| executor_runner.handle(message)),
        )
    }

    pub fn handle(&self, msg: ActionMessage) -> Result<(), ExecutorError> {
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
