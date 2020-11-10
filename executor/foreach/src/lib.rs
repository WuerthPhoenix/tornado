use log::*;
use std::collections::HashMap;
use std::sync::Arc;
use tornado_common_api::{Action, Value};
use tornado_common_parser::Parser;
use tornado_executor_common::{StatelessExecutor, ExecutorError};
use tornado_network_common::EventBus;
use std::rc::Rc;

const FOREACH_TARGET_KEY: &str = "target";
const FOREACH_ACTIONS_KEY: &str = "actions";
const FOREACH_ITEM_KEY: &str = "item";
const FOREACH_ACTION_ID_KEY: &str = "id";
const FOREACH_ACTION_PAYLOAD_KEY: &str = "payload";

pub struct ForEachExecutor {
    bus: Arc<dyn EventBus>,
}

impl std::fmt::Display for ForEachExecutor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("ForEachExecutor")?;
        Ok(())
    }
}

impl ForEachExecutor {
    pub fn new(bus: Arc<dyn EventBus>) -> Self {
        Self { bus }
    }
}

#[async_trait::async_trait(?Send)]
impl StatelessExecutor for ForEachExecutor {
    async fn execute(&self, action: Rc<Action>) -> Result<(), ExecutorError> {
        trace!("ForEachExecutor - received action: \n[{:?}]", action);

        match action.payload.get(FOREACH_TARGET_KEY) {
            Some(Value::Array(values)) => {
                let actions: Vec<Action> = match action.payload.get(FOREACH_ACTIONS_KEY) {
                    Some(Value::Array(actions)) => {
                        actions.iter().map(to_action).filter_map(Result::ok).collect()
                    }
                    _ => {
                        return Err(ExecutorError::MissingArgumentError {
                            message: format!(
                                "ForEachExecutor - No [{}] key found in payload",
                                FOREACH_ACTIONS_KEY
                            ),
                        })
                    }
                };

                actions.into_iter().for_each(|action| {
                    for value in values.iter() {
                        //let mut cloned_action = action.clone();
                        //cloned_action.payload.insert(FOREACH_ITEM_KEY.to_owned(), value.clone());

                        let mut item = HashMap::new();
                        item.insert(FOREACH_ITEM_KEY.to_owned(), value.clone());
                        if let Err(err) = resolve_action(&Value::Map(item), action.clone())
                            .map(|action| self.bus.publish_action(action)) {
                            warn!(
                                "ForEachExecutor - Error while executing internal action [{}]. Err: {}",
                                action.id, err
                            )
                        }
                    }
                });
                Ok(())
            }
            _ => Err(ExecutorError::MissingArgumentError {
                message: format!(
                    "ForEachExecutor - No [{}] key found in payload, or it's value is not an array",
                    FOREACH_TARGET_KEY
                ),
            }),
        }
    }
}

fn to_action(value: &Value) -> Result<Action, ExecutorError> {
    match value {
        Value::Map(action) => match action.get(FOREACH_ACTION_ID_KEY) {
            Some(Value::Text(id)) => match action.get(FOREACH_ACTION_PAYLOAD_KEY) {
                Some(Value::Map(payload)) => {
                    Ok(Action { id: id.to_owned(), payload: payload.clone() })
                }
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

fn resolve_action(item: &Value, mut action: Action) -> Result<Action, ExecutorError> {
    for (_key, element) in action.payload.iter_mut() {
        resolve_payload(item, element)?;
    }
    Ok(action)
}

fn resolve_payload(item: &Value, mut value: &mut Value) -> Result<(), ExecutorError> {
    match &mut value {
        Value::Text(text) => {
            if let Some(parse_result) = Parser::build_parser(text)
                .map_err(|err| ExecutorError::ActionExecutionError {
                    can_retry: false,
                    message: format!("Cannot build parser for [{}]. Err: {}", text, err),
                    code: None,
                })?
                .parse_value(item)
            {
                *value = parse_result.into_owned();
            }
        }
        Value::Array(values) => {
            for element in values.iter_mut() {
                resolve_payload(item, element)?;
            }
        }
        Value::Map(values) => {
            for (_key, element) in values.iter_mut() {
                resolve_payload(item, element)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::hash_map::Entry;
    use std::collections::HashMap;
    use std::sync::RwLock;
    use tornado_common_api::Number;
    use tornado_network_simple::SimpleEventBus;

    #[test]
    fn should_convert_value_to_action() {
        // Arrange
        let mut action_map = HashMap::new();
        action_map.insert("id".to_owned(), Value::Text("my_action".to_owned()));

        let mut payload_map = HashMap::new();
        payload_map.insert("key_one".to_owned(), Value::Array(vec![]));
        action_map.insert("payload".to_owned(), Value::Map(payload_map.clone()));

        let action_value = Value::Map(action_map);

        // Act
        let action = to_action(&action_value).unwrap();

        // Assert
        assert_eq!("my_action", action.id);
        assert_eq!(payload_map, action.payload);
    }

    #[test]
    fn to_action_should_fail_if_value_not_a_map() {
        // Act
        let result = to_action(&Value::Array(vec![]));

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn to_action_should_fail_if_missing_id() {
        // Arrange
        let mut action_map = HashMap::new();

        let mut payload_map = HashMap::new();
        payload_map.insert("key_one".to_owned(), Value::Array(vec![]));
        action_map.insert("payload".to_owned(), Value::Map(payload_map.clone()));

        let action_value = Value::Map(action_map);

        // Act
        let result = to_action(&action_value);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn to_action_should_fail_if_id_is_not_text() {
        // Arrange
        let mut action_map = HashMap::new();
        action_map.insert("id".to_owned(), Value::Number(Number::PosInt(1)));

        let mut payload_map = HashMap::new();
        payload_map.insert("key_one".to_owned(), Value::Array(vec![]));
        action_map.insert("payload".to_owned(), Value::Map(payload_map.clone()));

        let action_value = Value::Map(action_map);

        // Act
        let result = to_action(&action_value);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn to_action_should_fail_if_payload_is_missing() {
        // Arrange
        let mut action_map = HashMap::new();
        action_map.insert("id".to_owned(), Value::Text("my_action".to_owned()));

        let action_value = Value::Map(action_map);

        // Act
        let result = to_action(&action_value);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn to_action_should_fail_if_payload_is_not_map() {
        // Arrange
        let mut action_map = HashMap::new();
        action_map.insert("id".to_owned(), Value::Text("my_action".to_owned()));

        let mut payload_map = HashMap::new();
        payload_map.insert("payload".to_owned(), Value::Array(vec![]));

        let action_value = Value::Map(action_map);

        // Act
        let result = to_action(&action_value);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_execute_each_action_with_each_target_item() {
        // Arrange

        let execution_results = Arc::new(RwLock::new(HashMap::new()));

        let mut bus = SimpleEventBus::new();
        {
            let execution_results = execution_results.clone();
            bus.subscribe_to_action(
                "id_one",
                Box::new(move |action| {
                    let mut lock = execution_results.write().unwrap();
                    match lock.entry("id_one") {
                        Entry::Vacant(entry) => {
                            entry.insert(vec![action]);
                        }
                        Entry::Occupied(mut entry) => entry.get_mut().push(action),
                    }
                }),
            );
        };

        {
            let execution_results = execution_results.clone();
            bus.subscribe_to_action(
                "id_two",
                Box::new(move |action| {
                    let mut lock = execution_results.write().unwrap();
                    match lock.entry("id_two") {
                        Entry::Vacant(entry) => {
                            entry.insert(vec![action]);
                        }
                        Entry::Occupied(mut entry) => entry.get_mut().push(action),
                    }
                }),
            );
        };

        let mut executor = ForEachExecutor::new(Arc::new(bus));

        let mut action = Action::new("");
        action.payload.insert(
            "target".to_owned(),
            Value::Array(vec![
                Value::Text("first_item".to_owned()),
                Value::Text("second_item".to_owned()),
            ]),
        );

        let mut actions_array = vec![];

        {
            let mut action = HashMap::new();
            action.insert("id".to_owned(), Value::Text("id_one".to_owned()));

            let mut payload_one = HashMap::new();
            payload_one.insert("key_one".to_owned(), Value::Array(vec![]));
            payload_one.insert("item".to_owned(), Value::Text("${item}".to_owned()));
            action.insert("payload".to_owned(), Value::Map(payload_one.clone()));

            actions_array.push(Value::Map(action));
        }

        {
            let mut action = HashMap::new();
            action.insert("id".to_owned(), Value::Text("id_two".to_owned()));

            let mut payload_one = HashMap::new();
            payload_one.insert(
                "item_with_interpolation".to_owned(),
                Value::Text("a ${item} bb <${item}>".to_owned()),
            );
            action.insert("payload".to_owned(), Value::Map(payload_one.clone()));

            actions_array.push(Value::Map(action));
        }

        action.payload.insert("actions".to_owned(), Value::Array(actions_array));

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_ok());

        let lock = execution_results.read().unwrap();
        assert_eq!(2, lock.len());

        assert!(lock.contains_key("id_one"));
        assert!(lock.contains_key("id_two"));

        let action_one = lock.get("id_one").unwrap();
        assert_eq!(2, action_one.len());

        {
            let mut payload = HashMap::new();
            payload.insert("key_one".to_owned(), Value::Array(vec![]));
            payload.insert("item".to_owned(), Value::Text("first_item".to_owned()));
            assert_eq!(&Action::new_with_payload("id_one", payload), action_one.get(0).unwrap());
        }

        {
            let mut payload = HashMap::new();
            payload.insert("key_one".to_owned(), Value::Array(vec![]));
            payload.insert("item".to_owned(), Value::Text("second_item".to_owned()));
            assert_eq!(&Action::new_with_payload("id_one", payload), action_one.get(1).unwrap());
        }

        let action_two = lock.get("id_two").unwrap();
        assert_eq!(2, action_two.len());

        {
            let mut payload = HashMap::new();
            payload.insert(
                "item_with_interpolation".to_owned(),
                Value::Text("a first_item bb <first_item>".to_owned()),
            );
            assert_eq!(&Action::new_with_payload("id_two", payload), action_two.get(0).unwrap());
        }

        {
            let mut payload = HashMap::new();
            payload.insert(
                "item_with_interpolation".to_owned(),
                Value::Text("a second_item bb <second_item>".to_owned()),
            );
            assert_eq!(&Action::new_with_payload("id_two", payload), action_two.get(1).unwrap());
        }
    }

    #[test]
    fn should_ignore_failing_actions_and_execute_all_others() {
        // Arrange

        let execution_results = Arc::new(RwLock::new(HashMap::new()));

        let mut bus = SimpleEventBus::new();
        {
            let execution_results = execution_results.clone();
            bus.subscribe_to_action(
                "id_one",
                Box::new(move |action| {
                    let mut lock = execution_results.write().unwrap();
                    match lock.entry("id_one") {
                        Entry::Vacant(entry) => {
                            entry.insert(vec![action]);
                        }
                        Entry::Occupied(mut entry) => entry.get_mut().push(action),
                    }
                }),
            );
        };

        {
            let execution_results = execution_results.clone();
            bus.subscribe_to_action(
                "id_two",
                Box::new(move |action| {
                    let mut lock = execution_results.write().unwrap();
                    match lock.entry("id_two") {
                        Entry::Vacant(entry) => {
                            entry.insert(vec![action]);
                        }
                        Entry::Occupied(mut entry) => entry.get_mut().push(action),
                    }
                }),
            );
        };

        let mut executor = ForEachExecutor::new(Arc::new(bus));

        let mut action = Action::new("");
        action.payload.insert(
            "target".to_owned(),
            Value::Array(vec![
                Value::Text("first_item".to_owned()),
                Value::Text("second_item".to_owned()),
            ]),
        );

        let mut actions_array = vec![];

        {
            let mut action = HashMap::new();
            action.insert("id".to_owned(), Value::Text("id_one".to_owned()));
            actions_array.push(Value::Map(action));
        }

        {
            let mut action = HashMap::new();
            action.insert("id".to_owned(), Value::Text("id_two".to_owned()));

            let mut payload_one = HashMap::new();
            payload_one.insert("item".to_owned(), Value::Text("${item}".to_owned()));
            action.insert("payload".to_owned(), Value::Map(payload_one.clone()));

            actions_array.push(Value::Map(action));
        }

        action.payload.insert("actions".to_owned(), Value::Array(actions_array));

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_ok());

        let lock = execution_results.read().unwrap();
        assert_eq!(1, lock.len());

        assert!(lock.contains_key("id_two"));

        let action_two = lock.get("id_two").unwrap();
        assert_eq!(2, action_two.len());

        {
            let mut payload = HashMap::new();
            payload.insert("item".to_owned(), Value::Text("first_item".to_owned()));
            assert_eq!(&Action::new_with_payload("id_two", payload), action_two.get(0).unwrap());
        }

        {
            let mut payload = HashMap::new();
            payload.insert("item".to_owned(), Value::Text("second_item".to_owned()));
            assert_eq!(&Action::new_with_payload("id_two", payload), action_two.get(1).unwrap());
        }
    }

    #[test]
    fn should_resolve_complex_placeholders() {
        // Arrange

        let execution_results = Arc::new(RwLock::new(HashMap::new()));

        let mut bus = SimpleEventBus::new();
        {
            let execution_results = execution_results.clone();
            bus.subscribe_to_action(
                "id_one",
                Box::new(move |action| {
                    let mut lock = execution_results.write().unwrap();
                    match lock.entry("id_one") {
                        Entry::Vacant(entry) => {
                            entry.insert(vec![action]);
                        }
                        Entry::Occupied(mut entry) => entry.get_mut().push(action),
                    }
                }),
            );
        };

        let mut executor = ForEachExecutor::new(Arc::new(bus));

        let mut action = Action::new("");
        action.payload.insert(
            "target".to_owned(),
            Value::Array(vec![
                Value::Array(vec![
                    Value::Text("first".to_owned()),
                    Value::Text("second".to_owned()),
                ]),
                Value::Array(vec![
                    Value::Text("third".to_owned()),
                    Value::Text("fourth".to_owned()),
                ]),
            ]),
        );

        let mut actions_array = vec![];

        {
            let mut action = HashMap::new();
            action.insert("id".to_owned(), Value::Text("id_one".to_owned()));

            let mut payload_one = HashMap::new();
            payload_one
                .insert("value".to_owned(), Value::Text("${item[0]} + ${item[1]}".to_owned()));
            action.insert("payload".to_owned(), Value::Map(payload_one.clone()));

            actions_array.push(Value::Map(action));
        }

        action.payload.insert("actions".to_owned(), Value::Array(actions_array));

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_ok());

        let lock = execution_results.read().unwrap();
        assert_eq!(1, lock.len());

        assert!(lock.contains_key("id_one"));

        let action_two = lock.get("id_one").unwrap();
        assert_eq!(2, action_two.len());

        {
            let mut payload = HashMap::new();
            payload.insert("value".to_owned(), Value::Text("first + second".to_owned()));
            assert_eq!(&Action::new_with_payload("id_one", payload), action_two.get(0).unwrap());
        }

        {
            let mut payload = HashMap::new();
            payload.insert("value".to_owned(), Value::Text("third + fourth".to_owned()));
            assert_eq!(&Action::new_with_payload("id_one", payload), action_two.get(1).unwrap());
        }
    }

    #[test]
    fn should_resolve_recursive_placeholders_in_maps() {
        // Arrange

        let execution_results = Arc::new(RwLock::new(vec![]));

        let mut bus = SimpleEventBus::new();
        {
            let execution_results = execution_results.clone();
            bus.subscribe_to_action(
                "id_one",
                Box::new(move |action| {
                    let mut lock = execution_results.write().unwrap();
                    lock.push(action);
                }),
            );
        };

        let mut executor = ForEachExecutor::new(Arc::new(bus));

        let mut action = Action::new("");
        action.payload.insert(
            "target".to_owned(),
            Value::Array(vec![Value::Array(vec![
                Value::Text("first".to_owned()),
                Value::Text("second".to_owned()),
            ])]),
        );

        let mut actions_array = vec![];

        {
            let mut action = HashMap::new();
            action.insert("id".to_owned(), Value::Text("id_one".to_owned()));

            let mut inner_map = HashMap::new();
            inner_map.insert("value".to_owned(), Value::Text("${item[0]}".to_owned()));

            let mut payload_one = HashMap::new();
            payload_one.insert("inner".to_owned(), Value::Map(inner_map));
            action.insert("payload".to_owned(), Value::Map(payload_one.clone()));

            actions_array.push(Value::Map(action));
        }

        action.payload.insert("actions".to_owned(), Value::Array(actions_array));

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_ok());

        let lock = execution_results.read().unwrap();
        assert_eq!(1, lock.len());

        let value = lock.get(0).unwrap().payload.get("inner").unwrap().get_map().unwrap();
        let mut expected_map = HashMap::new();
        expected_map.insert("value".to_owned(), Value::Text("first".to_owned()));
        assert_eq!(&expected_map, value);
    }

    #[test]
    fn should_resolve_recursive_placeholders_in_arrays() {
        // Arrange

        let execution_results = Arc::new(RwLock::new(vec![]));

        let mut bus = SimpleEventBus::new();
        {
            let execution_results = execution_results.clone();
            bus.subscribe_to_action(
                "id_one",
                Box::new(move |action| {
                    let mut lock = execution_results.write().unwrap();
                    lock.push(action);
                }),
            );
        };

        let mut executor = ForEachExecutor::new(Arc::new(bus));

        let mut action = Action::new("");
        action.payload.insert(
            "target".to_owned(),
            Value::Array(vec![Value::Array(vec![
                Value::Text("first".to_owned()),
                Value::Text("second".to_owned()),
            ])]),
        );

        let mut actions_array = vec![];

        {
            let mut action = HashMap::new();
            action.insert("id".to_owned(), Value::Text("id_one".to_owned()));

            let mut inner_array = vec![];
            inner_array.push(Value::Text("${item[0]}".to_owned()));
            inner_array.push(Value::Text("${item[1]}".to_owned()));

            let mut payload_one = HashMap::new();
            payload_one.insert("inner".to_owned(), Value::Array(inner_array));
            action.insert("payload".to_owned(), Value::Map(payload_one.clone()));

            actions_array.push(Value::Map(action));
        }

        action.payload.insert("actions".to_owned(), Value::Array(actions_array));

        // Act
        let result = executor.execute(&action);

        // Assert
        assert!(result.is_ok());

        let lock = execution_results.read().unwrap();
        assert_eq!(1, lock.len());

        let value = lock.get(0).unwrap().payload.get("inner").unwrap().get_array().unwrap();
        let mut expected_array = vec![];
        expected_array.push(Value::Text("first".to_owned()));
        expected_array.push(Value::Text("second".to_owned()));
        assert_eq!(&expected_array, value);
    }
}
