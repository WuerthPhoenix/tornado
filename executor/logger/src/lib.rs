use log::*;
use tornado_common_api::Action;
use tornado_executor_common::{StatelessExecutor, ExecutorError};
use std::rc::Rc;

/// An executor that logs received actions at the 'info' level
#[derive(Default)]
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

#[async_trait::async_trait(?Send)]
impl StatelessExecutor for LoggerExecutor {
    async fn execute(&self, action: Rc<Action>) -> Result<(), ExecutorError> {
        info!("LoggerExecutor - received action: \n[{:?}]", action);
        Ok(())
    }
}
