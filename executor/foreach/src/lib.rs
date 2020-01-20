use log::*;
use tornado_common_api::{Action, Value};
use tornado_executor_common::{Executor, ExecutorError};
use std::sync::Arc;
use tornado_network_common::EventBus;

const FOREACH_TARGET_KEY: &str = "target";
const FOREACH_ACTIONS_KEY: &str = "actions";

pub struct ForEachExecutor {
    bus: Arc::<dyn EventBus>
}

impl ForEachExecutor {
    pub fn new(bus: Arc::<dyn EventBus>) -> Self {
        Self{bus}
    }
}

impl Executor for ForEachExecutor {
    fn execute(&mut self, action: Action) -> Result<(), ExecutorError> {
        trace!("ForEachExecutor - received action: \n[{:?}]", action);

        match action.payload.get(FOREACH_TARGET_KEY) {
            Some(Value::Array(values)) => {

            },
            _ => {
                return Err(ExecutorError::MissingArgumentError {
                    message: format!("ForEachExecutor - No [{}] key found in payload", FOREACH_TARGET_KEY)
                })
            }
        }

        Ok(())
    }
}
