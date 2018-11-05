#[macro_use]
extern crate log;
extern crate tornado_common_api;
extern crate tornado_executor_common;

use tornado_common_api::Action;
use tornado_executor_common::{Executor, ExecutorError};

/// An executor that logs received actions at info level.
#[derive(Default)]
pub struct LoggerExecutor {}

impl LoggerExecutor {
    pub fn new() -> LoggerExecutor {
        Default::default()
    }
}

impl Executor for LoggerExecutor {
    fn execute(&self, action: &Action) -> Result<(), ExecutorError> {
        info!("LoggerExecutor - received action: \n[{:#?}]", action);
        Ok(())
    }
}
