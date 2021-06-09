//! The action module contains the logic to build a Rule's actions based on the
//! Rule configuration.
//!
//! An *Action* is linked to the "actions" section of a Rule and determines the outcome
//! produced by a matching Event.

use crate::accessor::{Accessor, AccessorBuilder};
use crate::config::rule::Action as ConfigAction;
use crate::error::MatcherError;
use crate::interpolator::StringInterpolator;
use crate::model::{
    ActionMetaData, EnrichedValue, EnrichedValueContent, InternalEvent, ValueMetaData,
};
use std::collections::HashMap;
use tornado_common_api::Value;
use tornado_common_api::{Action, Number};

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
    pub fn build_all(
        &self,
        rule_name: &str,
        actions: &[ConfigAction],
    ) -> Result<Vec<ActionResolver>, MatcherError> {
        let mut matcher_actions = vec![];
        for action in actions {
            matcher_actions.push(self.build(rule_name, action)?);
        }
        Ok(matcher_actions)
    }

    /// Receives an Action as defined in a Rule and returns an ActionResolver.
    /// The ActionResolver contains the logic needed to build the final Action object, ready to be sent to the executors.
    pub fn build(
        &self,
        rule_name: &str,
        action: &ConfigAction,
    ) -> Result<ActionResolver, MatcherError> {
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

        Ok(matcher_action)
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
                let interpolator = StringInterpolator::build(text.as_str(), rule_name, accessor)?;
                if interpolator.is_interpolation_required() {
                    Ok(ActionValueProcessor::Interpolator(interpolator))
                } else {
                    Ok(ActionValueProcessor::Accessor(accessor.build(rule_name, text)?))
                }
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
    pub fn resolve(
        &self,
        event: &InternalEvent,
        extracted_vars: Option<&Value>,
    ) -> Result<Action, MatcherError> {
        let mut action = Action { trace_id: event.trace_id.clone(), id: self.id.to_owned(), payload: HashMap::new() };

        for (key, action_value_processor) in &self.payload {
            action.payload.insert(
                key.to_owned(),
                action_value_processor.process(&self.rule_name, &self.id, event, extracted_vars)?,
            );
        }

        Ok(action)
    }

    pub fn resolve_with_meta(
        &self,
        event: &InternalEvent,
        extracted_vars: Option<&Value>,
    ) -> Result<(Action, ActionMetaData), MatcherError> {
        let mut action = Action { trace_id: event.trace_id.clone(), id: self.id.to_owned(), payload: HashMap::new() };
        let mut action_meta = ActionMetaData { id: self.id.to_owned(), payload: HashMap::new() };

        for (key, action_value_processor) in &self.payload {
            let (value, value_enriched) = action_value_processor.process_enriched(
                &self.rule_name,
                &self.id,
                event,
                extracted_vars,
            )?;
            action.payload.insert(key.to_owned(), value);
            action_meta.payload.insert(key.to_owned(), value_enriched);
        }

        Ok((action, action_meta))
    }
}

#[derive(Debug, PartialEq)]
enum ActionValueProcessor {
    Accessor(Accessor),
    Null,
    Bool(bool),
    Number(Number),
    Interpolator(StringInterpolator),
    Array(Vec<ActionValueProcessor>),
    Map(HashMap<String, ActionValueProcessor>),
}

impl ActionValueProcessor {
    pub fn process(
        &self,
        rule_name: &str,
        action_id: &str,
        event: &InternalEvent,
        extracted_vars: Option<&Value>,
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
            ActionValueProcessor::Interpolator(interpolator) => {
                interpolator.render(event, extracted_vars).map(Value::Text)
            }
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

    pub fn process_enriched(
        &self,
        rule_name: &str,
        action_id: &str,
        event: &InternalEvent,
        extracted_vars: Option<&Value>,
    ) -> Result<(Value, EnrichedValue), MatcherError> {
        match self {
            ActionValueProcessor::Accessor(accessor) => {
                let value = accessor
                    .get(event, extracted_vars)
                    .ok_or(MatcherError::CreateActionError {
                        action_id: action_id.to_owned(),
                        rule_name: rule_name.to_owned(),
                        cause: format!("Accessor [{:?}] returned empty value.", accessor),
                    })?
                    .into_owned();
                Ok((
                    value.clone(),
                    EnrichedValue {
                        content: EnrichedValueContent::Single { content: value },
                        meta: ValueMetaData { is_leaf: true, modified: accessor.dynamic_value() },
                    },
                ))
            }
            ActionValueProcessor::Interpolator(interpolator) => {
                let value = interpolator.render(event, extracted_vars).map(Value::Text)?;
                Ok((
                    value.clone(),
                    EnrichedValue {
                        content: EnrichedValueContent::Single { content: value },
                        meta: ValueMetaData { is_leaf: true, modified: true },
                    },
                ))
            }
            ActionValueProcessor::Null => {
                let value = Value::Null;
                Ok((
                    value.clone(),
                    EnrichedValue {
                        content: EnrichedValueContent::Single { content: value },
                        meta: ValueMetaData { is_leaf: true, modified: false },
                    },
                ))
            }
            ActionValueProcessor::Number(number) => {
                let value = Value::Number(*number);
                Ok((
                    value.clone(),
                    EnrichedValue {
                        content: EnrichedValueContent::Single { content: value },
                        meta: ValueMetaData { is_leaf: true, modified: false },
                    },
                ))
            }
            ActionValueProcessor::Bool(boolean) => {
                let value = Value::Bool(*boolean);
                Ok((
                    value.clone(),
                    EnrichedValue {
                        content: EnrichedValueContent::Single { content: value },
                        meta: ValueMetaData { is_leaf: true, modified: false },
                    },
                ))
            }
            ActionValueProcessor::Map(payload) => {
                let mut processor_payload = HashMap::new();
                let mut processor_payload_enriched = HashMap::new();
                let mut modified = false;

                for (key, value) in payload {
                    let (value, enriched_value) =
                        value.process_enriched(rule_name, action_id, event, extracted_vars)?;
                    modified = modified || enriched_value.meta.modified;
                    processor_payload.insert(key.to_owned(), value);
                    processor_payload_enriched.insert(key.to_owned(), enriched_value);
                }

                let value = Value::Map(processor_payload);
                Ok((
                    value,
                    EnrichedValue {
                        content: EnrichedValueContent::Map { content: processor_payload_enriched },
                        meta: ValueMetaData { is_leaf: false, modified },
                    },
                ))
            }
            ActionValueProcessor::Array(values) => {
                let mut processor_values = vec![];
                let mut processor_payload_enriched = vec![];
                let mut modified = false;

                for value in values {
                    let (value, enriched_value) =
                        value.process_enriched(rule_name, action_id, event, extracted_vars)?;
                    modified = modified || enriched_value.meta.modified;
                    processor_values.push(value);
                    processor_payload_enriched.push(enriched_value);
                }
                let value = Value::Array(processor_values);
                Ok((
                    value,
                    EnrichedValue {
                        content: EnrichedValueContent::Array {
                            content: processor_payload_enriched,
                        },
                        meta: ValueMetaData { is_leaf: false, modified },
                    },
                ))
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
        let actions = ActionResolverBuilder::new().build_all("", &config).unwrap();

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
    fn action_resolver_builder_should_identify_whether_interpolation_is_required() {
        // Arrange
        let mut action = ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };

        action.payload.insert("constant".to_owned(), Value::Text("constant value".to_owned()));
        action.payload.insert("expression".to_owned(), Value::Text("${event.type}".to_owned()));
        action.payload.insert(
            "interpolation".to_owned(),
            Value::Text("event type: ${event.type}".to_owned()),
        );

        let config = vec![action];

        // Act
        let actions = ActionResolverBuilder::new().build_all("", &config).unwrap();

        // Assert
        assert_eq!(1, actions.len());
        assert_eq!("an_action_id", &actions.get(0).unwrap().id);

        let action_payload = &actions.get(0).unwrap().payload;
        assert_eq!(3, action_payload.len());

        assert_eq!(
            &ActionValueProcessor::Accessor(Accessor::Constant {
                value: Value::Text("constant value".to_owned())
            }),
            &action_payload["constant"]
        );

        assert_eq!(&ActionValueProcessor::Accessor(Accessor::Type), &action_payload["expression"]);

        match &action_payload["interpolation"] {
            ActionValueProcessor::Interpolator(..) => assert!(true),
            _ => assert!(false),
        }
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
            .insert("created_ms".to_owned(), Value::Text("${event.created_ms}".to_owned()));
        config_action
            .payload
            .insert("var_test_1".to_owned(), Value::Text("${_variables.test1}".to_owned()));
        config_action
            .payload
            .insert("var_test_2".to_owned(), Value::Text("${_variables.test2}".to_owned()));

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build_all(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_value".to_owned(), payload));

        let mut extracted_vars_inner = HashMap::new();
        extracted_vars_inner.insert("test1".to_owned(), Value::Text("var_test_1_value".to_owned()));
        extracted_vars_inner.insert("test2".to_owned(), Value::Text("var_test_2_value".to_owned()));

        let mut extracted_vars = HashMap::new();
        extracted_vars.insert("rule_for_test".to_owned(), Value::Map(extracted_vars_inner));

        // Act
        let result = matcher_action.resolve(&event, Some(&Value::Map(extracted_vars))).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(&"event_type_value", &result.payload.get("type").unwrap());
        assert_eq!(&"body_value", &result.payload.get("payload_body").unwrap());
        assert_eq!(&"subject_value", &result.payload.get("payload_subject").unwrap());
        assert_eq!(&"constant value", &result.payload.get("constant").unwrap());
        assert_eq!(&event.created_ms, result.payload.get("created_ms").unwrap());
        assert_eq!(&"var_test_1_value", &result.payload.get("var_test_1").unwrap());
        assert_eq!(&"var_test_2_value", &result.payload.get("var_test_2").unwrap());
        assert_eq!(&event.trace_id, &result.trace_id);
    }

    #[test]
    fn should_build_an_action_with_text_to_be_interpolated_in_config() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action
            .payload
            .insert("type".to_owned(), Value::Text("The event type is: ${event.type}".to_owned()));

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build_all(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("an_event_type_full_of_imagination".to_owned(), payload));

        // Act
        let result = matcher_action.resolve(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(
            &Value::Text("The event type is: an_event_type_full_of_imagination".to_owned()),
            result.payload.get("type").unwrap()
        );
    }

    #[test]
    fn should_build_an_action_with_bool_type_in_config() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action.payload.insert("type".to_owned(), Value::Bool(true));

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build_all(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_value".to_owned(), payload));

        // Act
        let result = matcher_action.resolve(&event, None).unwrap();

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
        let matcher_actions = ActionResolverBuilder::new().build_all(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_value".to_owned(), payload));

        // Act
        let result = matcher_action.resolve(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(&Value::Null, result.payload.get("type").unwrap());
    }

    #[test]
    fn should_build_an_action_with_number_type_in_config() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action.payload.insert("type".to_owned(), Value::Number(Number::PosInt(123456)));

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build_all(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_value".to_owned(), payload));

        // Act
        let result = matcher_action.resolve(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(&Value::Number(Number::PosInt(123456)), result.payload.get("type").unwrap());
    }

    #[test]
    fn should_build_an_action_with_array_type_in_config() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action.payload.insert(
            "type".to_owned(),
            Value::Array(vec![
                Value::Number(Number::Float(123456.0)),
                Value::Text("${event.type}".to_owned()),
                Value::Text("Event created on ${event.created_ms}".to_owned()),
            ]),
        );

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build_all(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));

        let event = Event::new_with_payload("event_type_value".to_owned(), payload);
        let created_ms = event.created_ms;
        let event = InternalEvent::new(event);

        // Act
        let result = matcher_action.resolve(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(
            &Value::Array(vec![
                Value::Number(Number::Float(123456.0)),
                Value::Text("event_type_value".to_owned()),
                Value::Text(format!("Event created on {}", created_ms))
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
                                     Value::Map(hashmap!["one".to_owned() => Value::Number(Number::Float(123456.0)),
                                            "two".to_owned() => Value::Text("${event.type}".to_owned())]
                                     ));

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build_all(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_value".to_owned(), payload));

        // Act
        let result = matcher_action.resolve(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(
            &Value::Map(hashmap!["one".to_owned() => Value::Number(Number::Float(123456.0)),
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
        let matcher_actions = ActionResolverBuilder::new().build_all(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut body = HashMap::new();
        body.insert("inner".to_owned(), Value::Text("inner_body_value".to_owned()));

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Map(body.clone()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_value".to_owned(), payload));

        // Act
        let result = matcher_action.resolve(&event, None).unwrap();

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
        let matcher_actions = ActionResolverBuilder::new().build_all(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("from_payload".to_owned()));
        payload.insert("some_null".to_owned(), Value::Null);

        let event = InternalEvent::new(Event::new_with_payload("event_type_value".to_owned(), payload));

        // Act
        let result = matcher_action.resolve(&event, None).unwrap();

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
        let matcher_actions = ActionResolverBuilder::new().build_all(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("from_payload".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_value".to_owned(), payload.clone()));

        // Act
        let result = matcher_action.resolve(&event, None).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(&Value::Map(payload), result.payload.get("event_payload").unwrap());
    }

    #[test]
    fn should_return_action_metadata_for_simple_action() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action
            .payload
            .insert("event_payload".to_owned(), Value::Text("${event.payload}".to_owned()));
        config_action
            .payload
            .insert("constant".to_owned(), Value::Text("Into The Great Wide Open".to_owned()));

        let rule_name = "rule_for_test";
        let action_resolver =
            ActionResolverBuilder::new().build(rule_name, &config_action).unwrap();

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("from_payload".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_value".to_owned(), payload.clone()));

        // Act
        let (action, action_meta_data) = action_resolver.resolve_with_meta(&event, None).unwrap();

        // Assert
        assert_eq!("an_action_id", &action.id);
        assert_eq!(&Value::Map(payload.clone()), action.payload.get("event_payload").unwrap());

        assert_eq!("an_action_id", &action_meta_data.id);

        let expected_action_meta_data = ActionMetaData {
            id: config_action.id.to_owned(),
            payload: hashmap! {
                "event_payload".to_owned() => EnrichedValue {
                    content: EnrichedValueContent::Single {
                        content: Value::Map(hashmap! {
                            "body".to_owned() => Value::Text("from_payload".to_owned())
                        })
                    },
                    meta: ValueMetaData {
                        modified: true,
                        is_leaf: true
                    },
                },
                "constant".to_owned() => EnrichedValue {
                    content: EnrichedValueContent::Single {
                        content: Value::Text("Into The Great Wide Open".to_owned())
                    },
                    meta: ValueMetaData {
                        modified: false,
                        is_leaf: true
                    },
                }
            },
        };
        assert_eq!(expected_action_meta_data, action_meta_data);
    }

    #[test]
    fn should_return_action_metadata_with_deep_map_value_resolution() {
        // Arrange
        let config_action = ConfigAction {
            id: "an_action_id".to_owned(),
            payload: hashmap! {
                "inner_map_static".to_owned() => Value::Map(
                    hashmap!{
                        "bool".to_owned() => Value::Bool(false)
                    }
                ),
                "inner_map_dynamic".to_owned() => Value::Map(
                    hashmap!{
                        "value".to_owned() => Value::Text("${event.payload.body}".to_owned())
                    }
                ),
            },
        };

        let rule_name = "rule_for_test";
        let action_resolver =
            ActionResolverBuilder::new().build(rule_name, &config_action).unwrap();

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("from_payload".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_value".to_owned(), payload));

        // Act
        let (action, action_meta_data) = action_resolver.resolve_with_meta(&event, None).unwrap();

        // Assert
        assert_eq!("an_action_id", &action.id);
        assert_eq!(
            &Value::Text("from_payload".to_owned()),
            action
                .payload
                .get("inner_map_dynamic")
                .unwrap()
                .get_map()
                .unwrap()
                .get("value")
                .unwrap()
        );

        assert_eq!("an_action_id", &action_meta_data.id);

        let expected_action_meta_data = ActionMetaData {
            id: config_action.id.to_owned(),
            payload: hashmap! {
                "inner_map_static".to_owned() => EnrichedValue {
                    content: EnrichedValueContent::Map {
                        content: hashmap! {
                            "bool".to_owned()  => EnrichedValue {
                                  content: EnrichedValueContent::Single { content: Value::Bool(false) },
                                  meta: ValueMetaData {
                                        modified: false,
                                        is_leaf: true
                                 },
                            }
                        }
                    },
                    meta: ValueMetaData {
                        modified: false,
                        is_leaf: false
                    },
                },
                "inner_map_dynamic".to_owned() => EnrichedValue {
                    content: EnrichedValueContent::Map {
                        content: hashmap! {
                            "value".to_owned()  => EnrichedValue {
                                  content: EnrichedValueContent::Single { content: Value::Text("from_payload".to_owned()) },
                                  meta: ValueMetaData {
                                        modified: true,
                                        is_leaf: true
                                 },
                            }
                        }
                    },
                    meta: ValueMetaData {
                        modified: true,
                        is_leaf: false
                    },
                },
            },
        };
        assert_eq!(expected_action_meta_data, action_meta_data);
    }

    #[test]
    fn should_return_action_metadata_with_deep_array_value_resolution() {
        // Arrange
        let config_action = ConfigAction {
            id: "an_action_id".to_owned(),
            payload: hashmap! {
                "inner_vec_static".to_owned() => Value::Array(vec![Value::Number(Number::PosInt(545))]),
                "inner_vec_dynamic".to_owned() => Value::Array(
                    vec![
                        Value::Map(hashmap!{
                                "value".to_owned() => Value::Text("${event.payload.body}".to_owned())
                        })
                    ]
                ),
            },
        };

        let rule_name = "rule_for_test";
        let action_resolver =
            ActionResolverBuilder::new().build(rule_name, &config_action).unwrap();

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("from_payload".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_value".to_owned(), payload));

        // Act
        let (action, action_meta_data) = action_resolver.resolve_with_meta(&event, None).unwrap();

        // Assert
        assert_eq!("an_action_id", &action.id);
        assert_eq!("an_action_id", &action_meta_data.id);

        let expected_action_meta_data = ActionMetaData {
            id: config_action.id.to_owned(),
            payload: hashmap! {
                "inner_vec_static".to_owned() => EnrichedValue {
                    content: EnrichedValueContent::Array {
                        content: vec![EnrichedValue {
                                  content: EnrichedValueContent::Single { content: Value::Number(Number::PosInt(545)) },
                                  meta: ValueMetaData {
                                        modified: false,
                                        is_leaf: true
                                 },
                            }]
                    },
                    meta: ValueMetaData {
                        modified: false,
                        is_leaf: false
                    },
                },
                "inner_vec_dynamic".to_owned() => EnrichedValue {
                    content: EnrichedValueContent::Array {
                        content: vec![
                            EnrichedValue {
                                content: EnrichedValueContent::Map {
                                    content: hashmap! {
                                        "value".to_owned()  => EnrichedValue {
                                              content: EnrichedValueContent::Single { content: Value::Text("from_payload".to_owned()) },
                                              meta: ValueMetaData {
                                                    modified: true,
                                                    is_leaf: true
                                             },
                                        }
                                    },
                                },
                                meta: ValueMetaData {
                                    modified: true,
                                    is_leaf: false
                                },
                            }
                        ]
                    },
                    meta: ValueMetaData {
                        modified: true,
                        is_leaf: false
                    },
                },
            },
        };
        assert_eq!(expected_action_meta_data, action_meta_data);
    }

    #[test]
    fn processed_action_should_have_same_trace_id_than_the_event() {
        // Arrange
        let config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build_all(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let event = InternalEvent::new(Event::new("event_type_value".to_owned()));

        // Act
        let result = matcher_action.resolve(&event, None).unwrap();

        // Assert
        assert_eq!(&event.trace_id, &result.trace_id);

    }
}
