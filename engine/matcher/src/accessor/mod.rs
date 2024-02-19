//! The accessor module contains the logic to extract data from an incoming Event.

use crate::{error::MatcherError, model::InternalEvent};
use log::*;
use serde_json::Value;
use std::borrow::Cow;
use tornado_common_parser::{Parser, ParserBuilder};

#[derive(Default)]
pub struct AccessorBuilder;

/// A builder for the Event Accessors
impl AccessorBuilder {
    pub fn new() -> AccessorBuilder {
        AccessorBuilder
    }

    pub fn build_from_value(
        &self,
        rule_name: &str,
        input: &Value,
    ) -> Result<Accessor, MatcherError> {
        match input {
            Value::String(text) => self.build(rule_name, text),
            _ => {
                Ok(Accessor { rule_name: rule_name.to_owned(), parser: Parser::Val(input.clone()) })
            }
        }
    }

    pub fn build(&self, rule_name: &str, input: &str) -> Result<Accessor, MatcherError> {
        trace!("AccessorBuilder - build: build accessor [{}] for rule [{}]", input, rule_name);

        let parser = ParserBuilder::engine_matcher(input);

        trace!(
            "AccessorBuilder - build: return accessor [{:?}] for input value [{}]",
            &parser,
            input
        );
        Ok(Accessor { rule_name: rule_name.to_owned(), parser: parser? })
    }
}

#[derive(Debug)]
pub struct Accessor {
    rule_name: String,
    parser: Parser,
}

impl Accessor {
    pub fn get<'o>(&'o self, data: &'o InternalEvent) -> Option<Cow<'o, Value>> {
        self.parser.parse_value(data, &self.rule_name)
    }

    /// Returns true if this Accessor returns a dynamic value that changes
    /// based on the event and extracted_vars content
    pub fn dynamic_value(&self) -> bool {
        !matches!(&self.parser, Parser::Val(_))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use serde_json::json;
    use tornado_common_api::{Event, Map, Value, ValueExt, WithEventData};
    use tornado_common_parser::ValueGetter;

    #[test]
    fn should_return_a_constant_value() {
        let accessor = Accessor {
            parser: Parser::Val(Value::String("constant_value".to_owned())),
            rule_name: "".to_owned(),
        };

        let event = json!(Event::new("event_type_string"));

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        assert_eq!("constant_value", result.as_ref());
        assert!(!accessor.dynamic_value());
    }

    #[test]
    fn should_not_trigger_a_constant_value() {
        let accessor = Accessor {
            parser: Parser::Val(Value::String("  constant_value  ".to_owned())),
            rule_name: "".to_owned(),
        };

        let event = json!(Event::new("event_type_string"));

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        assert_eq!("  constant_value  ", result.as_ref());
        assert!(!accessor.dynamic_value());
    }

    #[test]
    fn should_return_the_event_type() {
        let accessor = AccessorBuilder::new().build("", "${event.type}").unwrap();

        let event = json!(Event::new("event_type_string"));

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        assert_eq!("event_type_string", result.as_ref());
        assert!(accessor.dynamic_value());
    }

    #[test]
    fn should_return_the_event_created_ms() {
        let accessor = AccessorBuilder::new().build("", "${event.created_ms}").unwrap();

        let event = json!(Event::new("event_type_string"));

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        let created_ms = result.get_number().unwrap().clone();
        assert!(created_ms.is_u64());
        assert!(created_ms.as_u64().unwrap() > 0);
        assert!(accessor.dynamic_value());
    }

    #[test]
    fn should_return_value_from_payload_if_exists() {
        let accessor = Accessor {
            parser: ParserBuilder::default().build_parser("${event.payload.body}").unwrap(),
            rule_name: "rule".to_owned(),
        };

        let mut payload = Map::new();
        payload.insert("body".to_owned(), Value::String("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::String("subject_value".to_owned()));

        let event = json!(Event::new_with_payload("event_type_string", payload));

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        assert_eq!("body_value", result.as_ref());
        assert!(accessor.dynamic_value());
    }

    #[test]
    fn should_return_bool_value_from_payload() {
        // Arrange
        let accessor = Accessor {
            parser: ParserBuilder::default().build_parser("${event.payload.bool_true}").unwrap(),
            rule_name: "rule".to_owned(),
        };

        let mut payload = Map::new();
        payload.insert("bool_true".to_owned(), Value::Bool(true));
        payload.insert("bool_false".to_owned(), Value::Bool(false));

        let event = json!(Event::new_with_payload("event_type_string", payload));

        // Act
        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        // Assert
        assert_eq!(&true, result.as_ref());
        assert!(accessor.dynamic_value());
    }

    #[test]
    fn should_return_number_value_from_payload() {
        // Arrange
        let accessor = Accessor {
            parser: ParserBuilder::default().build_parser("${event.payload.num_555}").unwrap(),
            rule_name: "rule".to_owned(),
        };

        let mut payload = Map::new();
        payload.insert("num_555".to_owned(), json!(555.0));

        let event = json!(Event::new_with_payload("event_type_string", payload));

        // Act
        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        // Assert
        assert_eq!(555.0, result.as_ref().get_number().unwrap().as_f64().unwrap());
        assert!(accessor.dynamic_value());
    }

    #[test]
    fn should_return_non_text_nodes() {
        // Arrange
        let accessor = Accessor {
            parser: ParserBuilder::default().build_parser("${event.payload.body}").unwrap(),
            rule_name: "rule".to_owned(),
        };

        let mut body_payload = Map::new();
        body_payload.insert("first".to_owned(), Value::String("body_first_value".to_owned()));
        body_payload.insert("second".to_owned(), Value::String("body_second_value".to_owned()));

        let body_clone = body_payload.clone();

        let mut payload = Map::new();
        payload.insert("body".to_owned(), Value::Object(body_payload));

        let event = json!(Event::new_with_payload("event_type_string", payload));

        // Act
        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        // Assert
        assert_eq!(&Value::Object(body_clone), result.as_ref());
        assert!(accessor.dynamic_value());
    }

    #[test]
    fn should_return_value_from_nested_map_if_exists() {
        // Arrange
        let accessor = Accessor {
            parser: ParserBuilder::default().build_parser("${event.payload.body.first}").unwrap(),
            rule_name: "rule".to_owned(),
        };

        let mut body_payload = Map::new();
        body_payload.insert("first".to_owned(), Value::String("body_first_value".to_owned()));
        body_payload.insert("second".to_owned(), Value::String("body_second_value".to_owned()));

        let mut payload = Map::new();
        payload.insert("body".to_owned(), Value::Object(body_payload));

        let event = json!(Event::new_with_payload("event_type_string", payload));

        // Act
        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        // Assert
        assert_eq!("body_first_value", result.as_ref());
        assert!(accessor.dynamic_value());
    }

    #[test]
    fn should_return_value_from_nested_array_if_exists() {
        // Arrange
        let accessor = Accessor {
            parser: ParserBuilder::default().build_parser("${event.payload.body[1]}").unwrap(),
            rule_name: "rule".to_owned(),
        };

        let mut payload = Map::new();
        payload.insert(
            "body".to_owned(),
            Value::Array(vec![
                Value::String("body_first_value".to_owned()),
                Value::String("body_second_value".to_owned()),
            ]),
        );

        let event = json!(Event::new_with_payload("event_type_string", payload));

        // Act
        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        // Assert
        assert_eq!("body_second_value", result.as_ref());
        assert!(accessor.dynamic_value());
    }

    #[test]
    fn should_accept_double_quotas_delimited_keys() {
        // Arrange
        let accessor = AccessorBuilder::new()
            .build("rule", r#"${event.payload.body."second.with.dot"}"#)
            .unwrap();

        let mut body_payload = Map::new();
        body_payload.insert("first".to_owned(), Value::String("body_first_value".to_owned()));
        body_payload
            .insert("second.with.dot".to_owned(), Value::String("body_second_value".to_owned()));

        let mut payload = Map::new();
        payload.insert("body".to_owned(), Value::Object(body_payload));

        let event = json!(Event::new_with_payload("event_type_string", payload));

        // Act
        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        // Assert
        assert_eq!("body_second_value", result.as_ref());
        assert!(accessor.dynamic_value());
    }

    #[test]
    fn should_return_none_from_payload_if_not_exists() {
        let accessor = Accessor {
            parser: ParserBuilder::default().build_parser("${event.payload.date}").unwrap(),
            rule_name: "rule".to_owned(),
        };

        let mut payload = Map::new();
        payload.insert("body".to_owned(), Value::String("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::String("subject_value".to_owned()));

        let event = json!(Event::new_with_payload("event_type_string", payload));

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event);

        assert!(result.is_none());
    }

    #[test]
    fn should_return_the_entire_event() {
        let accessor = AccessorBuilder::new().build("", "${event}").unwrap();

        let mut payload = Map::new();
        payload.insert("body".to_owned(), Value::String("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::String("subject_value".to_owned()));

        let mut event = json!(Event::new_with_payload("event_type_string", payload.clone()));
        event.add_to_metadata("body".to_owned(), Value::String("body_value".to_owned())).unwrap();
        event
            .add_to_metadata("subject".to_owned(), Value::String("subject_value".to_owned()))
            .unwrap();

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        let event_value: Value = event.clone();
        assert_eq!(&event_value, result.as_ref());

        let json_from_result = serde_json::to_string(result.as_ref()).unwrap();
        let event_from_result: Value = serde_json::from_str(&json_from_result).unwrap();
        assert_eq!(event, event_from_result);

        assert!(accessor.dynamic_value());
    }

    #[test]
    fn should_return_the_entire_payload() {
        let accessor = Accessor {
            parser: ParserBuilder::default().build_parser("${event.payload}").unwrap(),
            rule_name: "rule".to_owned(),
        };

        let mut payload = Map::new();
        payload.insert("body".to_owned(), Value::String("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::String("subject_value".to_owned()));

        let event = json!(Event::new_with_payload("event_type_string", payload.clone()));

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        assert_eq!(&json!(payload), result.as_ref());
    }

    #[test]
    fn should_return_value_from_extracted_var() {
        let builder = AccessorBuilder::new();
        let value = "${_variables.body}".to_owned();

        let accessor = builder.build("rule1", &value).unwrap();

        let event = json!(Event::new("event_type_string"));
        let mut extracted_vars_inner = Map::new();
        extracted_vars_inner.insert("body".to_owned(), Value::String("body_value".to_owned()));
        extracted_vars_inner
            .insert("subject".to_owned(), Value::String("subject_value".to_owned()));

        let mut extracted_vars = Map::new();
        extracted_vars.insert("rule1".to_owned(), Value::Object(extracted_vars_inner));
        let mut extracted_vars = Value::Object(extracted_vars);

        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        assert_eq!("body_value", result.as_ref());
        assert!(accessor.dynamic_value());
    }

    #[test]
    fn should_return_value_from_extracted_var_of_current_rule() {
        let builder = AccessorBuilder::new();
        let value = "${_variables.body}".to_owned();

        let accessor = builder.build("current_rule_name", &value).unwrap();

        let event = json!(Event::new("event_type_string"));
        let mut extracted_vars_current = Map::new();
        extracted_vars_current.insert("body".to_owned(), Value::String("current_body".to_owned()));
        extracted_vars_current
            .insert("subject".to_owned(), Value::String("current_subject".to_owned()));

        let mut extracted_vars_custom = Map::new();
        extracted_vars_custom.insert("body".to_owned(), Value::String("custom_body".to_owned()));
        extracted_vars_custom
            .insert("subject".to_owned(), Value::String("custom_subject".to_owned()));

        let mut extracted_vars = Map::new();
        extracted_vars
            .insert("current_rule_name".to_owned(), Value::Object(extracted_vars_current));
        extracted_vars.insert("custom_rule_name".to_owned(), Value::Object(extracted_vars_custom));
        let mut extracted_vars = Value::Object(extracted_vars);

        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        assert_eq!("current_body", result.as_ref());
    }

    #[test]
    fn should_return_value_from_extracted_var_of_custom_rule() {
        let builder = AccessorBuilder::new();
        let value = "${_variables.custom_rule_name.body}".to_owned();

        let accessor = builder.build("current_rule_name", &value).unwrap();

        let event = json!(Event::new("event_type_string"));

        let mut extracted_vars_current = Map::new();
        extracted_vars_current.insert("body".to_owned(), Value::String("current_body".to_owned()));
        extracted_vars_current
            .insert("subject".to_owned(), Value::String("current_subject".to_owned()));

        let mut extracted_vars_custom = Map::new();
        extracted_vars_custom.insert("body".to_owned(), Value::String("custom_body".to_owned()));
        extracted_vars_custom
            .insert("subject".to_owned(), Value::String("custom_subject".to_owned()));

        let mut extracted_vars = Map::new();
        extracted_vars
            .insert("current_rule_name".to_owned(), Value::Object(extracted_vars_current));
        extracted_vars.insert("custom_rule_name".to_owned(), Value::Object(extracted_vars_custom));
        let mut extracted_vars = Value::Object(extracted_vars);

        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        assert_eq!("custom_body", result.as_ref());
    }

    #[test]
    fn should_return_none_if_no_match() {
        let accessor = Accessor {
            rule_name: "rule1".to_owned(),
            parser: ParserBuilder::default().build_parser("${event.payload.body}").unwrap(),
        };

        let event = json!(Event::new("event_type_string"));

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event);

        assert!(result.is_none());
    }

    #[test]
    fn builder_should_return_constant_accessor() {
        let builder = AccessorBuilder::new();
        let value = "constant_value".to_owned();

        let accessor = builder.build("", &value).unwrap();

        match accessor {
            Accessor { parser: Parser::Val(inner_value), rule_name: _ } => {
                assert_eq!("constant_value", &inner_value);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn builder_should_return_payload_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        match accessor {
            Accessor { parser: Parser::Exp { keys }, rule_name } => {
                assert_eq!(
                    vec![
                        ValueGetter::Map { key: "event".to_owned() },
                        ValueGetter::Map { key: "payload".to_owned() },
                    ],
                    keys
                );
                assert_eq!(rule_name, "");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn builder_should_return_payload_with_inner_key() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload.key}".to_owned();

        let accessor = builder.build("rule", &value).unwrap();

        match accessor {
            Accessor { parser: Parser::Exp { keys }, rule_name } => {
                assert_eq!(
                    vec![
                        ValueGetter::Map { key: "event".to_owned() },
                        ValueGetter::Map { key: "payload".to_owned() },
                        ValueGetter::Map { key: "key".to_owned() },
                    ],
                    keys
                );
                assert_eq!(rule_name, "rule");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn builder_should_return_payload_accessor_with_nested_keys() {
        let builder = AccessorBuilder::new();
        let value = r#"${event.payload.first.second."th. ird"."four"}"#.to_owned();

        let accessor = builder.build("rule", &value).unwrap();

        match accessor {
            Accessor { parser: Parser::Exp { keys }, rule_name } => {
                assert_eq!(
                    vec![
                        ValueGetter::Map { key: "event".to_owned() },
                        ValueGetter::Map { key: "payload".to_owned() },
                        ValueGetter::Map { key: "first".to_owned() },
                        ValueGetter::Map { key: "second".to_owned() },
                        ValueGetter::Map { key: "th. ird".to_owned() },
                        ValueGetter::Map { key: "four".to_owned() },
                    ],
                    keys
                );
                assert_eq!(rule_name, "rule");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn builder_should_return_current_rule_extracted_var_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${_variables.key}".to_owned();

        let accessor = builder.build("current_rule_name", &value).unwrap();

        match accessor {
            Accessor { parser: Parser::Custom { .. }, rule_name } => {
                assert_eq!(rule_name, "current_rule_name");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn builder_should_return_custom_rule_extracted_var_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${_variables.custom_rule.key}".to_owned();

        let accessor = builder.build("current_rule_name", &value).unwrap();

        match accessor {
            Accessor { parser: Parser::Custom { .. }, rule_name } => {
                assert_eq!(rule_name, "current_rule_name");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn builder_should_return_event_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event}".to_owned();

        let accessor = builder.build("rule", &value).unwrap();

        let mut payload = Map::new();
        payload.insert("body".to_owned(), Value::String("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::String("subject_value".to_owned()));
        let event = json!(Event::new_with_payload("event_type_string", payload));

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        let event_value: Value = event.clone();
        assert_eq!(&event_value, result.as_ref());

        match accessor {
            Accessor { parser: Parser::Exp { keys }, rule_name } => {
                assert_eq!(vec![ValueGetter::Map { key: "event".to_owned() },], keys);
                assert_eq!(rule_name, "rule");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn builder_should_return_payload_accessor_with_expected_key() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload.body}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        let mut payload = Map::new();
        payload.insert("body".to_owned(), Value::String("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::String("subject_value".to_owned()));

        let event = json!(Event::new_with_payload("event_type_string", payload));

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        assert_eq!("body_value", result.as_ref());
    }

    #[test]
    fn builder_should_return_error_if_unknown_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${events.types}".to_owned();

        let accessor = builder.build("", &value);

        assert!(&accessor.is_err());

        match accessor.err().unwrap() {
            MatcherError::ConfigurationError { .. } => {}
            _ => unreachable!(),
        };
    }

    #[test]
    fn builder_should_return_error_if_unknown_accessor_with_event_suffix() {
        let builder = AccessorBuilder::new();
        let value = "${events}".to_owned();

        let accessor = builder.build("", &value);

        assert!(&accessor.is_err());

        match accessor.err().unwrap() {
            MatcherError::ConfigurationError { .. } => {}
            _ => unreachable!(),
        };
    }

    #[test]
    fn accessor_should_return_the_entire_payload_if_empty_payload_key() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload.}";

        let accessor = builder.build("", value);

        assert!(&accessor.is_ok());
    }

    #[test]
    fn should_return_nested_values_from_extracted_var() {
        // Arrange
        let builder = AccessorBuilder::new();
        let map_accessor = builder.build("", "${_variables.rule1.body.map.key_1}").unwrap();
        let array_accessor = builder.build("", "${_variables.rule1.body.array[0]}").unwrap();

        let event = json!(Event::new("event_type_string"));

        let mut map = Map::new();
        map.insert("key_1".to_owned(), Value::String("first_from_map".to_owned()));

        let mut body = Map::new();
        body.insert("map".to_owned(), Value::Object(map));
        body.insert(
            "array".to_owned(),
            Value::Array(vec![Value::String("first_from_array".to_owned())]),
        );

        let mut extracted_vars_inner = Map::new();
        extracted_vars_inner.insert("body".to_owned(), Value::Object(body));

        let mut extracted_vars = Map::new();
        extracted_vars.insert("rule1".to_owned(), Value::Object(extracted_vars_inner));
        let mut extracted_vars = Value::Object(extracted_vars);

        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();

        // Act
        let map_result = map_accessor.get(&internal_event).unwrap();
        let array_result = array_accessor.get(&internal_event).unwrap();

        // Assert
        assert_eq!("first_from_map", map_result.as_ref());
        assert_eq!("first_from_array", array_result.as_ref());
    }

    #[test]
    fn should_build_a_constant_parser_val_accessor_for_expression_who_start_with_item() {
        // Arrange
        let builder = AccessorBuilder::new();
        let value = "${item.body}".to_owned();

        // Act
        let accessor = builder.build("rule_name", &value).unwrap();

        // Assert
        match accessor {
            Accessor { rule_name: _, parser: Parser::Val(Value::String(inner_value)) } => {
                assert_eq!(&value, &inner_value);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn should_get_a_constant_val_expression_starting_with_item() {
        // Arrange
        let builder = AccessorBuilder::new();
        let value = "${item.body}".to_owned();
        let internal_event = InternalEvent {
            event: &Default::default(),
            extracted_variables: &mut Default::default(),
        };

        // Act
        let accessor = builder.build("rule_name", &value).unwrap();

        // Assert
        let parsed_value = accessor.get(&internal_event);
        assert_eq!(parsed_value.unwrap().as_ref(), &Value::String(value));
    }

    #[test]
    fn should_build_a_constant_parser_val_for_interpolated_ignored_expression_item() {
        // Arrange
        let builder = AccessorBuilder::new();
        let value = "my body is ${item.body}!".to_owned();
        let internal_event = InternalEvent {
            event: &Default::default(),
            extracted_variables: &mut Default::default(),
        };

        // Act
        let accessor = builder.build("rule_name", &value).unwrap();

        // Assert
        let parsed_value = accessor.get(&internal_event);
        assert_eq!(parsed_value.unwrap().as_ref(), &Value::String(value));
    }

    #[test]
    fn should_build_a_constant_parser_val_for_ignored_expression_array_access() {
        // Arrange
        let builder = AccessorBuilder::new();
        let value = "my body is ${item[0].body}!".to_owned();
        let internal_event = InternalEvent {
            event: &Default::default(),
            extracted_variables: &mut Default::default(),
        };

        // Act
        let accessor = builder.build("rule_name", &value).unwrap();

        // Assert
        let parsed_value = accessor.get(&internal_event);
        assert_eq!(parsed_value.unwrap().as_ref(), &Value::String(value));
    }

    #[test]
    fn metadata_accessor_should_return_with_expected_key() {
        // Arrange
        let builder = AccessorBuilder::new();
        let value = "${event.metadata.tenant_id}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        let mut event = json!(Event::new("event_type_string"));
        event
            .add_to_metadata(
                "tenant_id".to_owned(),
                Value::String("A_TENANT_ID_FROM_METADATA".to_owned()),
            )
            .unwrap();

        // Act
        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        // Assert
        assert_eq!("A_TENANT_ID_FROM_METADATA", result.as_ref());
    }

    #[test]
    fn metadata_accessor_should_return_entire_object() {
        // Arrange
        let builder = AccessorBuilder::new();
        let value = "${event.metadata}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        let mut event = json!(Event::new("event_type_string"));
        event
            .add_to_metadata(
                "tenant_id".to_owned(),
                Value::String("A_TENANT_ID_FROM_METADATA".to_owned()),
            )
            .unwrap();

        // Act
        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event).unwrap();

        // Assert
        assert_eq!(event.metadata().unwrap(), result.as_ref());
    }

    #[test]
    fn metadata_accessor_should_return_none_if_no_value() {
        // Arrange
        let builder = AccessorBuilder::new();
        let value = "${event.metadata.tenant_id}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        let mut event = json!(Event::new("event_type_string"));
        event.add_to_metadata("other".to_owned(), Value::String("something".to_owned())).unwrap();

        // Act
        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();
        let result = accessor.get(&internal_event);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn accessor_should_not_trim_the_values() {
        let accessor_1 = AccessorBuilder::new().build("", "  ${event.type}  ").unwrap();
        let accessor_2 = AccessorBuilder::new().build("", "  CONSTANT  ").unwrap();

        let event = json!(Event::new(" event_type_string "));

        let mut extracted_vars = Value::Null;
        let internal_event: InternalEvent = (&event, &mut extracted_vars).into();

        let result_1 = accessor_1.get(&internal_event).unwrap();
        let result_2 = accessor_2.get(&internal_event).unwrap();

        assert_eq!("   event_type_string   ", result_1.as_ref());
        assert_eq!("  CONSTANT  ", result_2.as_ref());
    }
}
