use log::*;
use std::sync::Arc;
use tornado_common_api::{Action, Value};
use tornado_executor_common::{Executor, ExecutorError};
use tornado_network_common::EventBus;

const FOREACH_TARGET_KEY: &str = "target";
const FOREACH_ACTIONS_KEY: &str = "actions";
const FOREACH_ITEM_KEY: &str = "item";

pub struct ForEachExecutor {
    bus: Arc<dyn EventBus>,
}

impl ForEachExecutor {
    pub fn new(bus: Arc<dyn EventBus>) -> Self {
        Self { bus }
    }
}

impl Executor for ForEachExecutor {
    fn execute(&mut self, mut action: Action) -> Result<(), ExecutorError> {
        trace!("ForEachExecutor - received action: \n[{:?}]", action);

        match action.payload.remove(FOREACH_TARGET_KEY) {
            Some(Value::Array(values)) => {
                let actions: Vec<Action> = match action.payload.remove(FOREACH_ACTIONS_KEY) {
                    Some(Value::Array(actions)) => actions
                        .into_iter()
                        .map(to_action)
                        .filter_map(Result::ok)
                        .collect(),
                    _ => {
                        return Err(ExecutorError::MissingArgumentError {
                            message: format!(
                                "ForEachExecutor - No [{}] key found in payload",
                                FOREACH_ACTIONS_KEY
                            ),
                        })
                    }
                };

                actions.iter().for_each(|action| {
                    for value in values.iter() {
                        let mut cloned_action = action.clone();
                        cloned_action.payload.insert(FOREACH_ITEM_KEY.to_owned(), value.clone());
                        self.bus.publish_action(cloned_action);
                    }
                });
                Ok(())
            }
            _ => Err(ExecutorError::MissingArgumentError {
                message: format!(
                    "ForEachExecutor - No [{}] key found in payload",
                    FOREACH_TARGET_KEY
                ),
            }),
        }
    }
}

fn to_action(value: Value) -> Result<Action, ExecutorError> {
    match value {
        Value::Map(mut action) => match action.remove("id") {
            Some(Value::Text(id)) => match action.remove("payload") {
                Some(Value::Map(payload)) => Ok(Action { id, payload }),
                _ => {
                    let message =
                        "ForEachExecutor - Not valid action format: Missing payload.".to_owned();
                    warn!("{}", message);
                    Err(ExecutorError::MissingArgumentError { message })
                }
            },
            _ => {
                let message = "ForEachExecutor - Not valid action format: Missing id.".to_owned();
                warn!("{}", message);
                Err(ExecutorError::MissingArgumentError { message })
            }
        },
        _ => {
            let message = "ForEachExecutor - Not valid action format".to_owned();
            warn!("{}", message);
            Err(ExecutorError::MissingArgumentError { message })
        }
    }
}
