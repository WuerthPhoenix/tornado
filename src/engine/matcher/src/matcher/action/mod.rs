//! The action module contains the logic to build a Rule's actions based on the
//! Rule configuration.
//!
//! An *Action* is linked to the "actions" section of a Rule and determines the outcome
//! produced by a matching Event.

use crate::accessor::{Accessor, AccessorBuilder};
use crate::config::rule::Action as ConfigAction;
use crate::error::MatcherError;
use crate::model::InternalEvent;
use std::collections::HashMap;
use tornado_common_api::Action;
use tornado_common_api::Value;

#[derive(Default)]
pub struct ActionResolverBuilder {
    accessor: AccessorBuilder,
}

/// The ActionResolver builder
impl ActionResolverBuilder {
    pub fn new() -> ActionResolverBuilder {
        ActionResolverBuilder { accessor: AccessorBuilder::new() }
    }

    /// Receives an array of Actions as defined in a Rule and returns an array of ActionResolver elements.
    /// Each ActionResolver is linked to an input Action definition and contains the logic needed to build
    /// the final Action object, ready to be sent to the executors.
    pub fn build(
        &self,
        rule_name: &str,
        actions: &[ConfigAction],
    ) -> Result<Vec<ActionResolver>, MatcherError> {
        let mut matcher_actions = vec![];

        for action in actions {
            let mut matcher_action = ActionResolver {
                rule_name: rule_name.to_owned(),
                id: action.id.to_owned(),
                payload: HashMap::new(),
            };

            for (payload_key, payload_value) in &action.payload {
                matcher_action
                    .payload
                    //.insert(payload_key.to_owned(), self.accessor.build(rule_name, payload_value)?);
                    .insert(
                        payload_key.to_owned(),
                        ActionResolverBuilder::build_action_value_processor(
                            &rule_name,
                            &self.accessor,
                            payload_value,
                        )?,
                    );
            }

            matcher_actions.push(matcher_action);
        }

        Ok(matcher_actions)
    }

    fn build_action_value_processor(
        rule_name: &str,
        accessor: &AccessorBuilder,
        value: &Value,
    ) -> Result<ActionValueProcessor, MatcherError> {
        match value {
            Value::Map(payload) => {
                let mut processor_payload = HashMap::new();
                for (key, value) in payload {
                    processor_payload.insert(
                        key.to_owned(),
                        ActionResolverBuilder::build_action_value_processor(
                            rule_name, accessor, value,
                        )?,
                    );
                }
                Ok(ActionValueProcessor::Map(processor_payload))
            }
            Value::Array(values) => {
                let mut processor_values = vec![];
                for value in values {
                    processor_values.push(ActionResolverBuilder::build_action_value_processor(
                        rule_name, accessor, value,
                    )?)
                }
                Ok(ActionValueProcessor::Array(processor_values))
            }
            Value::Text(text) => {
                Ok(ActionValueProcessor::Accessor(accessor.build(rule_name, text)?))
            }
            Value::Bool(boolean) => Ok(ActionValueProcessor::Bool(*boolean)),
            Value::Number(number) => Ok(ActionValueProcessor::Number(*number)),
            Value::Null => Ok(ActionValueProcessor::Null),
        }
    }
}

/// An Action resolver creates Actions from a InternalEvent.
pub struct ActionResolver {
    rule_name: String,
    id: String,
    payload: HashMap<String, ActionValueProcessor>,
}

impl ActionResolver {
    /// Builds an Action by extracting the required data from the InternalEvent.
    /// The outcome is a fully resolved Action ready to be processed by the executors.
    pub fn execute(
        &self,
        event: &InternalEvent,
        extracted_vars: Option<&HashMap<String, Value>>,
    ) -> Result<Action, MatcherError> {
        let mut action = Action { id: self.id.to_owned(), payload: HashMap::new() };

        for (key, action_value_processor) in &self.payload {
            action.payload.insert(
                key.to_owned(),
                action_value_processor.process(&self.rule_name, &self.id, event, extracted_vars)?,
            );
        }

        Ok(action)
    }
}

#[derive(Debug, PartialEq)]
enum ActionValueProcessor {
    Accessor(Accessor),
    Null,
    Bool(bool),
    Number(f64),
    //Text(String),
    Array(Vec<ActionValueProcessor>),
    Map(HashMap<String, ActionValueProcessor>),
}

impl ActionValueProcessor {
    pub fn process(
        &self,
        rule_name: &str,
        action_id: &str,
        event: &InternalEvent,
        extracted_vars: Option<&HashMap<String, Value>>,
    ) -> Result<Value, MatcherError> {
        match self {
            ActionValueProcessor::Accessor(accessor) => Ok(accessor
                .get(event, extracted_vars)
                .ok_or(MatcherError::CreateActionError {
                    action_id: action_id.to_owned(),
                    rule_name: rule_name.to_owned(),
                    cause: format!("Accessor [{:?}] returned empty value.", accessor),
                })?
                .into_owned()),
            //ActionValueProcessor::Text(text) => Ok(Value::Text(text.to_owned())),
            ActionValueProcessor::Null => Ok(Value::Null),
            ActionValueProcessor::Number(number) => Ok(Value::Number(*number)),
            ActionValueProcessor::Bool(boolean) => Ok(Value::Bool(*boolean)),
            ActionValueProcessor::Map(payload) => {
                let mut processor_payload = HashMap::new();
                for (key, value) in payload {
                    processor_payload.insert(
                        key.to_owned(),
                        value.process(rule_name, action_id, event, extracted_vars)?,
                    );
                }
                Ok(Value::Map(processor_payload))
            }
            ActionValueProcessor::Array(values) => {
                let mut processor_values = vec![];
                for value in values {
                    processor_values.push(value.process(
                        rule_name,
                        action_id,
                        event,
                        extracted_vars,
                    )?)
                }
                Ok(Value::Array(processor_values))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::accessor::Accessor;
    use maplit::*;
    use std::collections::HashMap;
    use tornado_common_api::{Event, Payload};

    #[test]
    fn should_build_a_matcher_action() {
        // Arrange
        let mut action = ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        let value = "constant value".to_owned();
        action.payload.insert("key".to_owned(), Value::Text(value.clone()));

        let config = vec![action];

        // Act
        let actions = ActionResolverBuilder::new().build("", &config).unwrap();

        // Assert
        assert_eq!(1, actions.len());
        assert_eq!("an_action_id", &actions.get(0).unwrap().id);

        let action_payload = &actions.get(0).unwrap().payload;
        assert_eq!(1, action_payload.len());
        assert!(action_payload.contains_key("key"));
        assert_eq!(
            &ActionValueProcessor::Accessor(Accessor::Constant { value: Value::Text(value) }),
            action_payload.get("key").unwrap()
        )
    }

    #[test]
    fn should_build_an_action() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action.payload.insert("type".to_owned(), Value::Text("${event.type}".to_owned()));
        config_action
            .payload
            .insert("payload_body".to_owned(), Value::Text("${event.payload.body}".to_owned()));
        config_action.payload.insert(
            "payload_subject".to_owned(),
            Value::Text("${event.payload.subject}".to_owned()),
        );
        config_action
            .payload
            .insert("constant".to_owned(), Value::Text("constant value".to_owned()));
        config_action
            .payload
            .insert("created_ts".to_owned(), Value::Text("${event.created_ts}".to_owned()));
        config_action
            .payload
            .insert("var_test_1".to_owned(), Value::Text("${_variables.test1}".to_owned()));
        config_action
            .payload
            .insert("var_test_2".to_owned(), Value::Text("${_variables.test2}".to_owned()));

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ts: "123456".to_owned(),
            payload,
        });
        let mut extracted_vars = HashMap::new();
        extracted_vars
            .insert("rule_for_test.test1".to_owned(), Value::Text("var_test_1_value".to_owned()));
        extracted_vars
            .insert("rule_for_test.test2".to_owned(), Value::Text("var_test_2_value".to_owned()));

        // Act
        let result = matcher_action.execute(&event, Some(&extracted_vars)).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(&"event_type_value", &result.payload.get("type").unwrap());
        assert_eq!(&"body_value", &result.payload.get("payload_body").unwrap());
        assert_eq!(&"subject_value", &result.payload.get("payload_subject").unwrap());
        assert_eq!(&"constant value", &result.payload.get("constant").unwrap());
        assert_eq!(&"123456", &result.payload.get("created_ts").unwrap());
        assert_eq!(&"var_test_1_value", &result.payload.get("var_test_1").unwrap());
        assert_eq!(&"var_test_2_value", &result.payload.get("var_test_2").unwrap());
    }

    #[test]
    fn should_build_an_action_with_bool_type_in_config() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action.payload.insert("type".to_owned(), Value::Bool(true));

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ts: "123456".to_owned(),
            payload,
        });

        // Act
        let result = matcher_action.execute(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(&Value::Bool(true), result.payload.get("type").unwrap());
    }

    #[test]
    fn should_build_an_action_with_null_type_in_config() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action.payload.insert("type".to_owned(), Value::Null);

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ts: "123456".to_owned(),
            payload,
        });

        // Act
        let result = matcher_action.execute(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(&Value::Null, result.payload.get("type").unwrap());
    }

    #[test]
    fn should_build_an_action_with_number_type_in_config() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action.payload.insert("type".to_owned(), Value::Number(123456.0));

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ts: "123456".to_owned(),
            payload,
        });

        // Act
        let result = matcher_action.execute(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(&Value::Number(123456.0), result.payload.get("type").unwrap());
    }

    #[test]
    fn should_build_an_action_with_array_type_in_config() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action.payload.insert(
            "type".to_owned(),
            Value::Array(vec![Value::Number(123456.0), Value::Text("${event.type}".to_owned())]),
        );

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ts: "123456".to_owned(),
            payload,
        });

        // Act
        let result = matcher_action.execute(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(
            &Value::Array(vec![
                Value::Number(123456.0),
                Value::Text("event_type_value".to_owned())
            ]),
            result.payload.get("type").unwrap()
        );
    }

    #[test]
    fn should_build_an_action_with_map_type_in_config() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action.payload.insert("type".to_owned(),
                                     Value::Map(hashmap!["one".to_owned() => Value::Number(123456.0),
                                            "two".to_owned() => Value::Text("${event.type}".to_owned())]
                                     ));

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ts: "123456".to_owned(),
            payload,
        });

        // Act
        let result = matcher_action.execute(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(
            &Value::Map(hashmap!["one".to_owned() => Value::Number(123456.0),
                                            "two".to_owned() => Value::Text("event_type_value".to_owned())]),
            result.payload.get("type").unwrap()
        );
    }

    #[test]
    fn should_build_an_action_with_maps_in_payload() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action
            .payload
            .insert("payload_body".to_owned(), Value::Text("${event.payload.body}".to_owned()));
        config_action.payload.insert(
            "payload_body_inner".to_owned(),
            Value::Text("${event.payload.body.inner}".to_owned()),
        );

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut body = HashMap::new();
        body.insert("inner".to_owned(), Value::Text("inner_body_value".to_owned()));

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Map(body.clone()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ts: "123456".to_owned(),
            payload,
        });

        // Act
        let result = matcher_action.execute(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!("inner_body_value", result.payload.get("payload_body_inner").unwrap());
        assert_eq!(&Value::Map(body.clone()), result.payload.get("payload_body").unwrap());
    }

    #[test]
    fn should_put_the_whole_event_in_the_payload() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action.payload.insert("event".to_owned(), Value::Text("${event}".to_owned()));

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("from_payload".to_owned()));
        payload.insert("some_null".to_owned(), Value::Null);

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ts: "123456".to_owned(),
            payload,
        });

        // Act
        let result = matcher_action.execute(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);

        let event_value: Value = event.clone().into();
        assert_eq!(&event_value, result.payload.get("event").unwrap());
    }

    #[test]
    fn should_put_the_whole_event_payload_in_the_action_payload() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action
            .payload
            .insert("event_payload".to_owned(), Value::Text("${event.payload}".to_owned()));

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("from_payload".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ts: "123456".to_owned(),
            payload: payload.clone(),
        });

        // Act
        let result = matcher_action.execute(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(&Value::Map(payload), result.payload.get("event_payload").unwrap());
    }
}
