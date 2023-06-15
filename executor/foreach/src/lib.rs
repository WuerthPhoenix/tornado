use log::*;
use std::sync::Arc;
use tornado_common::actors::message::ActionMessage;
use tornado_common_api::{Action, Map, Payload, TracedAction, Value};
use tornado_common_parser::{ParserBuilder, FOREACH_ITEM_KEY};
use tornado_executor_common::{ExecutorError, StatelessExecutor};
use tornado_network_common::EventBus;
use tracing::instrument;

const FOREACH_TARGET_KEY: &str = "target";
const FOREACH_ACTIONS_KEY: &str = "actions";
const FOREACH_ACTION_ID_KEY: &str = "id";
const FOREACH_ACTION_PAYLOAD_KEY: &str = "payload";

pub struct ForEachExecutor {
    bus: Arc<dyn EventBus>,
}

pub struct Params<'a> {
    values: &'a Vec<Value>,
    actions: Vec<Action>,
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

    #[instrument(level = "debug", name = "Extract parameters for Executor", skip_all)]
    fn extract_params_from_payload<'a>(
        &self,
        payload: &'a Payload,
        action_created_ms: u64,
    ) -> Result<Params<'a>, ExecutorError> {
        let values = match payload.get(FOREACH_TARGET_KEY) {
            Some(Value::Array(values)) => values,
            Some(_) => {
                return Err(ExecutorError::MissingArgumentError {
                    message: format!(
                        "ForEachExecutor - Key [{}] is not an array",
                        FOREACH_TARGET_KEY
                    ),
                })
            }
            _ => {
                return Err(ExecutorError::MissingArgumentError {
                    message: format!(
                        "ForEachExecutor - No [{}] key found in payload.",
                        FOREACH_TARGET_KEY
                    ),
                })
            }
        };

        let actions: Vec<_> = match payload.get(FOREACH_ACTIONS_KEY) {
            Some(Value::Array(actions)) => actions
                .iter()
                .filter_map(|value| to_action(value, action_created_ms).ok())
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

        Ok(Params { values, actions })
    }
}

#[async_trait::async_trait(?Send)]
impl StatelessExecutor for ForEachExecutor {
    #[tracing::instrument(level = "info", skip_all, err, fields(otel.name = format!("Execute Action: {}", &action.id).as_str(), otel.kind = "Consumer"))]
    async fn execute(&self, action: Arc<Action>) -> Result<(), ExecutorError> {
        trace!("ForEachExecutor - received action: \n[{:?}]", action);

        let Params { values, actions } =
            self.extract_params_from_payload(&action.payload, action.created_ms)?;

        let execution_span = tracing::debug_span!(
            "ForEachExecutor",
            otel.name =
                format!("Execute {} Actions for {} Values", actions.len(), values.len()).as_str()
        );

        actions.into_iter().for_each(|action| {
            for value in values.iter() {
                //let mut cloned_action = action.clone();
                //cloned_action.payload.insert(FOREACH_ITEM_KEY.to_owned(), value.clone());

                let mut item = Map::new();
                item.insert(FOREACH_ITEM_KEY.to_owned(), value.clone());

                let result = resolve_action(&Value::Object(item), action.clone()).map(|action| {
                    self.bus.publish_action(ActionMessage(TracedAction {
                        action: Arc::new(action),
                        span: execution_span.clone(),
                    }))
                });

                if let Err(err) = result {
                    warn!(
                        "ForEachExecutor - Error while executing internal action [{}]. Err: {:?}",
                        action.id, err
                    )
                }
            }
        });
        Ok(())
    }
}

fn to_action(value: &Value, action_created_ms: u64) -> Result<Action, ExecutorError> {
    let message = match (value.get(FOREACH_ACTION_ID_KEY), value.get(FOREACH_ACTION_PAYLOAD_KEY)) {
        (Some(Value::String(id)), Some(Value::Object(payload))) => {
            return Ok(Action::new_with_payload_and_created_ms(
                id.to_owned(),
                payload.clone(),
                action_created_ms,
            ))
        }
        (Some(Value::String(_)), _) => {
            "ForEachExecutor - Not valid action format: Missing payload."
        }
        (_, Some(Value::Object(_))) => "ForEachExecutor - Not valid action format: Missing id.",
        _ => "ForEachExecutor - Not valid action format",
    };

    warn!("{}", message);
    Err(ExecutorError::MissingArgumentError { message: message.to_owned() })
}

fn resolve_action(item: &Value, mut action: Action) -> Result<Action, ExecutorError> {
    for (_key, element) in action.payload.iter_mut() {
        resolve_payload(item, element)?;
    }
    Ok(action)
}

fn resolve_payload(item: &Value, mut value: &mut Value) -> Result<(), ExecutorError> {
    match &mut value {
        Value::String(text) => {
            if let Some(parse_result) = ParserBuilder::default()
                .build_parser(text)
                .map_err(|err| ExecutorError::ActionExecutionError {
                    can_retry: false,
                    message: format!("Cannot build parser for [{}]. Err: {:?}", text, err),
                    code: None,
                    data: Default::default(),
                })?
                .parse_value(item, &())
            {
                *value = parse_result.into_owned();
            }
        }
        Value::Array(values) => {
            for element in values.iter_mut() {
                resolve_payload(item, element)?;
            }
        }
        Value::Object(values) => {
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
    use serde_json::json;
    use std::ops::Deref;
    use std::{
        collections::{hash_map::Entry, HashMap},
        sync::RwLock,
    };
    use tornado_common_api::ValueExt;
    use tornado_network_simple::SimpleEventBus;

    #[test]
    fn should_convert_value_to_action() {
        // Arrange
        let mut action_map = Map::new();
        action_map.insert("id".to_owned(), Value::String("my_action".to_owned()));

        let mut payload_map = Map::new();
        payload_map.insert("key_one".to_owned(), Value::Array(vec![]));
        action_map.insert("payload".to_owned(), Value::Object(payload_map.clone()));

        let action_value = Value::Object(action_map);
        let created_ms = 123456;

        // Act
        let action = to_action(&action_value, created_ms).unwrap();

        // Assert
        assert_eq!("my_action", action.id);
        assert_eq!(payload_map, action.payload);
        assert_eq!(created_ms, action.created_ms);
    }

    #[test]
    fn to_action_should_fail_if_value_not_a_map() {
        // Act
        let result = to_action(&Value::Array(vec![]), 123456);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn to_action_should_fail_if_missing_id() {
        // Arrange
        let mut action_map = Map::new();

        let mut payload_map = Map::new();
        payload_map.insert("key_one".to_owned(), Value::Array(vec![]));
        action_map.insert("payload".to_owned(), Value::Object(payload_map.clone()));

        let action_value = Value::Object(action_map);
        let created_ms = 123456;

        // Act
        let result = to_action(&action_value, created_ms);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn to_action_should_fail_if_id_is_not_text() {
        // Arrange
        let mut action_map = Map::new();
        action_map.insert("id".to_owned(), json!(1usize));

        let mut payload_map = Map::new();
        payload_map.insert("key_one".to_owned(), Value::Array(vec![]));
        action_map.insert("payload".to_owned(), Value::Object(payload_map.clone()));

        let action_value = Value::Object(action_map);
        let created_ms = 123456;

        // Act
        let result = to_action(&action_value, created_ms);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn to_action_should_fail_if_payload_is_missing() {
        // Arrange
        let mut action_map = Map::new();
        action_map.insert("id".to_owned(), Value::String("my_action".to_owned()));

        let action_value = Value::Object(action_map);

        // Act
        let result = to_action(&action_value, 123456);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn to_action_should_fail_if_payload_is_not_map() {
        // Arrange
        let mut action_map = Map::new();
        action_map.insert("id".to_owned(), Value::String("my_action".to_owned()));

        let mut payload_map = Map::new();
        payload_map.insert("payload".to_owned(), Value::Array(vec![]));

        let action_value = Value::Object(action_map);

        // Act
        let result = to_action(&action_value, 123456);

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_execute_each_action_with_each_target_item() {
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

        let executor = ForEachExecutor::new(Arc::new(bus));

        let created_ms = 123456;
        let mut action = Action::new_with_payload_and_created_ms("", Payload::new(), created_ms);
        action.payload.insert(
            "target".to_owned(),
            Value::Array(vec![
                Value::String("first_item".to_owned()),
                Value::String("second_item".to_owned()),
            ]),
        );

        let mut actions_array = vec![];

        {
            let mut action = Map::new();
            action.insert("id".to_owned(), Value::String("id_one".to_owned()));

            let mut payload_one = Map::new();
            payload_one.insert("key_one".to_owned(), Value::Array(vec![]));
            payload_one.insert("item".to_owned(), Value::String("${item}".to_owned()));
            action.insert("payload".to_owned(), Value::Object(payload_one.clone()));

            actions_array.push(Value::Object(action));
        }

        {
            let mut action = Map::new();
            action.insert("id".to_owned(), Value::String("id_two".to_owned()));

            let mut payload_one = Map::new();
            payload_one.insert(
                "item_with_interpolation".to_owned(),
                Value::String("a ${item} bb <${item}>".to_owned()),
            );
            action.insert("payload".to_owned(), Value::Object(payload_one.clone()));

            actions_array.push(Value::Object(action));
        }

        action.payload.insert("actions".to_owned(), Value::Array(actions_array));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        assert!(result.is_ok());

        let lock = execution_results.read().unwrap();
        assert_eq!(2, lock.len());

        assert!(lock.contains_key("id_one"));
        assert!(lock.contains_key("id_two"));

        let action_one = lock.get("id_one").unwrap();
        assert_eq!(2, action_one.len());

        {
            let mut payload = Map::new();
            payload.insert("key_one".to_owned(), Value::Array(vec![]));
            payload.insert("item".to_owned(), Value::String("first_item".to_owned()));
            assert_eq!(
                &Action::new_with_payload_and_created_ms("id_one", payload, created_ms),
                action_one.get(0).unwrap().0.action.deref()
            );
        }

        {
            let mut payload = Map::new();
            payload.insert("key_one".to_owned(), Value::Array(vec![]));
            payload.insert("item".to_owned(), Value::String("second_item".to_owned()));
            assert_eq!(
                &Action::new_with_payload_and_created_ms("id_one", payload, created_ms),
                action_one.get(1).unwrap().0.action.deref()
            );
        }

        let action_two = lock.get("id_two").unwrap();
        assert_eq!(2, action_two.len());

        {
            let mut payload = Map::new();
            payload.insert(
                "item_with_interpolation".to_owned(),
                Value::String("a first_item bb <first_item>".to_owned()),
            );
            assert_eq!(
                &Action::new_with_payload_and_created_ms("id_two", payload, created_ms),
                action_two.get(0).unwrap().0.action.deref()
            );
        }

        {
            let mut payload = Map::new();
            payload.insert(
                "item_with_interpolation".to_owned(),
                Value::String("a second_item bb <second_item>".to_owned()),
            );
            assert_eq!(
                &Action::new_with_payload_and_created_ms("id_two", payload, created_ms),
                action_two.get(1).unwrap().0.action.deref()
            );
        }
    }

    #[tokio::test]
    async fn should_ignore_failing_actions_and_execute_all_others() {
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

        let executor = ForEachExecutor::new(Arc::new(bus));

        let created_ms = 123456;
        let mut action = Action::new_with_payload_and_created_ms("", Payload::new(), created_ms);
        action.payload.insert(
            "target".to_owned(),
            Value::Array(vec![
                Value::String("first_item".to_owned()),
                Value::String("second_item".to_owned()),
            ]),
        );

        let mut actions_array = vec![];

        {
            let mut action = Map::new();
            action.insert("id".to_owned(), Value::String("id_one".to_owned()));
            actions_array.push(Value::Object(action));
        }

        {
            let mut action = Map::new();
            action.insert("id".to_owned(), Value::String("id_two".to_owned()));

            let mut payload_one = Map::new();
            payload_one.insert("item".to_owned(), Value::String("${item}".to_owned()));
            action.insert("payload".to_owned(), Value::Object(payload_one.clone()));

            actions_array.push(Value::Object(action));
        }

        action.payload.insert("actions".to_owned(), Value::Array(actions_array));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        assert!(result.is_ok());

        let lock = execution_results.read().unwrap();
        assert_eq!(1, lock.len());

        assert!(lock.contains_key("id_two"));

        let action_two = lock.get("id_two").unwrap();
        assert_eq!(2, action_two.len());

        {
            let mut payload = Map::new();
            payload.insert("item".to_owned(), Value::String("first_item".to_owned()));
            assert_eq!(
                &Action::new_with_payload_and_created_ms("id_two", payload, created_ms),
                action_two.get(0).unwrap().0.action.deref()
            );
        }

        {
            let mut payload = Map::new();
            payload.insert("item".to_owned(), Value::String("second_item".to_owned()));
            assert_eq!(
                &Action::new_with_payload_and_created_ms("id_two", payload, created_ms),
                action_two.get(1).unwrap().0.action.deref()
            );
        }
    }

    #[tokio::test]
    async fn should_resolve_complex_placeholders() {
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

        let executor = ForEachExecutor::new(Arc::new(bus));

        let created_ms = 123456;
        let mut action = Action::new_with_payload_and_created_ms("", Payload::new(), created_ms);
        action.payload.insert(
            "target".to_owned(),
            Value::Array(vec![
                Value::Array(vec![
                    Value::String("first".to_owned()),
                    Value::String("second".to_owned()),
                ]),
                Value::Array(vec![
                    Value::String("third".to_owned()),
                    Value::String("fourth".to_owned()),
                ]),
            ]),
        );

        let mut actions_array = vec![];

        {
            let mut action = Map::new();
            action.insert("id".to_owned(), Value::String("id_one".to_owned()));

            let mut payload_one = Map::new();
            payload_one
                .insert("value".to_owned(), Value::String("${item[0]} + ${item[1]}".to_owned()));
            action.insert("payload".to_owned(), Value::Object(payload_one.clone()));

            actions_array.push(Value::Object(action));
        }

        action.payload.insert("actions".to_owned(), Value::Array(actions_array));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        assert!(result.is_ok());

        let lock = execution_results.read().unwrap();
        assert_eq!(1, lock.len());

        assert!(lock.contains_key("id_one"));

        let action_two = lock.get("id_one").unwrap();
        assert_eq!(2, action_two.len());

        {
            let mut payload = Map::new();
            payload.insert("value".to_owned(), Value::String("first + second".to_owned()));
            assert_eq!(
                &Action::new_with_payload_and_created_ms("id_one", payload, created_ms),
                action_two.get(0).unwrap().0.action.deref()
            );
        }

        {
            let mut payload = Map::new();
            payload.insert("value".to_owned(), Value::String("third + fourth".to_owned()));
            assert_eq!(
                &Action::new_with_payload_and_created_ms("id_one", payload, created_ms),
                action_two.get(1).unwrap().0.action.deref()
            );
        }
    }

    #[tokio::test]
    async fn should_resolve_recursive_placeholders_in_maps() {
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

        let executor = ForEachExecutor::new(Arc::new(bus));

        let mut action = Action::new("");
        action.payload.insert(
            "target".to_owned(),
            Value::Array(vec![Value::Array(vec![
                Value::String("first".to_owned()),
                Value::String("second".to_owned()),
            ])]),
        );

        let mut actions_array = vec![];

        {
            let mut action = Map::new();
            action.insert("id".to_owned(), Value::String("id_one".to_owned()));

            let mut inner_map = Map::new();
            inner_map.insert("value".to_owned(), Value::String("${item[0]}".to_owned()));

            let mut payload_one = Map::new();
            payload_one.insert("inner".to_owned(), Value::Object(inner_map));
            action.insert("payload".to_owned(), Value::Object(payload_one.clone()));

            actions_array.push(Value::Object(action));
        }

        action.payload.insert("actions".to_owned(), Value::Array(actions_array));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        assert!(result.is_ok());

        let lock = execution_results.read().unwrap();
        assert_eq!(1, lock.len());

        let value = lock.get(0).unwrap().0.action.payload.get("inner").unwrap().get_map().unwrap();
        let mut expected_map = Map::new();
        expected_map.insert("value".to_owned(), Value::String("first".to_owned()));
        assert_eq!(&expected_map, value);
    }

    #[tokio::test]
    async fn should_resolve_recursive_placeholders_in_arrays() {
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

        let executor = ForEachExecutor::new(Arc::new(bus));

        let mut action = Action::new("");
        action.payload.insert(
            "target".to_owned(),
            Value::Array(vec![Value::Array(vec![
                Value::String("first".to_owned()),
                Value::String("second".to_owned()),
            ])]),
        );

        let mut actions_array = vec![];

        {
            let mut action = Map::new();
            action.insert("id".to_owned(), Value::String("id_one".to_owned()));

            let inner_array = vec![
                Value::String("${item[0]}".to_owned()),
                Value::String("${item[1]}".to_owned()),
            ];

            let mut payload_one = Map::new();
            payload_one.insert("inner".to_owned(), Value::Array(inner_array));
            action.insert("payload".to_owned(), Value::Object(payload_one.clone()));

            actions_array.push(Value::Object(action));
        }

        action.payload.insert("actions".to_owned(), Value::Array(actions_array));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        assert!(result.is_ok());

        let lock = execution_results.read().unwrap();
        assert_eq!(1, lock.len());

        let value =
            lock.get(0).unwrap().0.action.payload.get("inner").unwrap().get_array().unwrap();
        let expected_array =
            vec![Value::String("first".to_owned()), Value::String("second".to_owned())];
        assert_eq!(&expected_array, value);
    }
}
