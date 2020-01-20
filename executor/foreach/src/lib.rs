use log::*;
use tornado_common_api::{Action, Value};
use tornado_executor_common::{Executor, ExecutorError};
use std::sync::Arc;
use tornado_network_common::EventBus;

const FOREACH_TARGET_KEY: &str = "target";
const FOREACH_ACTIONS_KEY: &str = "actions";
const FOREACH_ITEM_KEY: &str = "item";

pub struct ForEachExecutor {
    bus: Arc::<dyn EventBus>
}

impl ForEachExecutor {
    pub fn new(bus: Arc::<dyn EventBus>) -> Self {
        Self{bus}
    }
}

impl Executor for ForEachExecutor {
    fn execute(&mut self, mut action: Action) -> Result<(), ExecutorError> {
        trace!("ForEachExecutor - received action: \n[{:?}]", action);

        match action.payload.remove(FOREACH_TARGET_KEY) {
            Some(Value::Array(values)) => {

                let actions = match action.payload.remove(FOREACH_ACTIONS_KEY) {
                    Some(Value::Array(actions)) => actions,
                    _ => {
                        return Err(ExecutorError::MissingArgumentError {
                            message: format!("ForEachExecutor - No [{}] key found in payload", FOREACH_ACTIONS_KEY)
                        })
                    }
                };

                for value in values.into_iter() {

                }
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

fn to_action(mut value: Value) -> Result<Action, ExecutorError> {
    match value {
        Value::Map(mut action) => {
            match action.remove("id") {
                Some(Value::Text(id)) => {
                    match action.remove("payload") {
                        Some(Value::Map(payload)) => {
                            Ok(Action{
                                id,
                                payload
                            })
                        },
                        _ => {
                            Err(ExecutorError::MissingArgumentError {
                                message: "ForEachExecutor - Not valid action format: Missing payload.".to_owned()
                            })
                        }
                    }
                },
                _ => {
                    Err(ExecutorError::MissingArgumentError {
                        message: "ForEachExecutor - Not valid action format: Missing id.".to_owned()
                    })
                }
            }
        },
        _ => {
            Err(ExecutorError::MissingArgumentError {
                message: "ForEachExecutor - Not valid action format".to_owned()
            })
        }
    }
}