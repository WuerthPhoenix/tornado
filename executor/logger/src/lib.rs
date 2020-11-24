use log::*;
use tornado_common_api::Action;
use tornado_executor_common::{Executor, ExecutorError};

/// An executor that logs received actions at the 'info' level
#[derive(Default, Clone)]
pub struct LoggerExecutor {}

impl LoggerExecutor {
    pub fn new() -> LoggerExecutor {
        Default::default()
    }
}

impl std::fmt::Display for LoggerExecutor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("LoggerExecutor")?;
        Ok(())
    }
}

impl Executor for LoggerExecutor {
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError> {
        info!("LoggerExecutor - received action: \n[{:?}]", action);
        Ok(())
    }
}
