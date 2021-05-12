use log::*;
use std::sync::Arc;
use tornado_common_api::Action;
use tornado_executor_common::{ExecutorError, StatelessExecutor};

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

#[async_trait::async_trait(?Send)]
impl StatelessExecutor for LoggerExecutor {
    async fn execute(&self, action: Arc<Action>) -> Result<(), ExecutorError> {
        info!("LoggerExecutor - received action: \n[{:?}]", action);
        Ok(())
    }
}
