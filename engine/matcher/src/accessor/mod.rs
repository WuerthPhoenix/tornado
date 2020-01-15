//! The accessor module contains the logic to extract data from an incoming Event.

use crate::error::MatcherError;
use crate::model::InternalEvent;
use crate::validator::id::IdValidator;
use log::*;
use std::borrow::Cow;
use std::collections::HashMap;
use tornado_common_api::Value;
use tornado_common_parser::{Parser, EXPRESSION_END_DELIMITER, EXPRESSION_START_DELIMITER};

pub struct AccessorBuilder {
    id_validator: IdValidator,
    start_delimiter: &'static str,
    end_delimiter: &'static str,
}

impl Default for AccessorBuilder {
    fn default() -> Self {
        AccessorBuilder {
            id_validator: IdValidator::new(),
            start_delimiter: EXPRESSION_START_DELIMITER,
            end_delimiter: EXPRESSION_END_DELIMITER,
        }
    }
}

const CURRENT_RULE_EXTRACTED_VAR_SUFFIX: &str = "_variables.";
const EVENT_KEY: &str = "event";
const EVENT_TYPE_KEY: &str = "event.type";
const EVENT_CREATED_MS_KEY: &str = "event.created_ms";
const EVENT_PAYLOAD_SUFFIX: &str = "event.payload";

/// A builder for the Event Accessors
impl AccessorBuilder {
    pub fn new() -> AccessorBuilder {
        Default::default()
    }

    pub fn build_from_value(
        &self,
        rule_name: &str,
        input: &Value,
    ) -> Result<Accessor, MatcherError> {
        match input {
            Value::Text(text) => self.build(rule_name, text),
            _ => Ok(Accessor::Constant { value: input.clone() }),
        }
    }

    /// Returns an Accessor instance based on its string definition.
    /// E.g.:
    /// - "${event}": returns the entire Event instance
    /// - "${event.type}": returns an instance of Accessor::Type
    /// - "${event.created_ms}": returns an instance of Accessor::CreatedTs
    /// - "${event.payload}": returns the entire Payload of the Event
    /// - "${event.payload.body}": returns an instance of Accessor::Payload that returns the value of the entry with the key "body" from the event payload
    /// - "event.type": returns an instance of Accessor::Constant that always returns the String "event.type"
    pub fn build(&self, rule_name: &str, input: &str) -> Result<Accessor, MatcherError> {
        trace!("AccessorBuilder - build: build accessor [{}] for rule [{}]", input, rule_name);
        let result = match input.trim() {
            value
                if value.starts_with(self.start_delimiter)
                    && value.ends_with(self.end_delimiter) =>
            {
                let path =
                    &value[self.start_delimiter.len()..(value.len() - self.end_delimiter.len())];
                match path.trim() {
                    EVENT_KEY => Ok(Accessor::Event {}),
                    EVENT_TYPE_KEY => Ok(Accessor::Type {}),
                    EVENT_CREATED_MS_KEY => Ok(Accessor::CreatedMs {}),
                    val if (val.starts_with(&format!("{}.", EVENT_PAYLOAD_SUFFIX))
                        || val.eq(EVENT_PAYLOAD_SUFFIX)) =>
                    {
                        let key = val[EVENT_PAYLOAD_SUFFIX.len()..].trim();
                        let parser = Parser::build_parser(&format!(
                            "{}{}{}",
                            EXPRESSION_START_DELIMITER, key, EXPRESSION_END_DELIMITER
                        ))?;
                        Ok(Accessor::Payload { parser })
                    }
                    val if val.starts_with(CURRENT_RULE_EXTRACTED_VAR_SUFFIX) => {
                        let key = val[CURRENT_RULE_EXTRACTED_VAR_SUFFIX.len()..].trim();

                        let keys: Vec<&str> = key.split('.').collect();
                        if keys.len() == 1 {
                            let variable_name = keys[0];
                            self.id_validator.validate_extracted_var_from_accessor(
                                variable_name,
                                value,
                                rule_name,
                            )?;
                            Ok(Accessor::ExtractedVar {
                                rule_name: rule_name.to_owned(),
                                key: variable_name.to_owned(),
                            })
                        } else if keys.len() == 2 {
                            let variable_rule = keys[0];
                            let variable_name = keys[1];
                            self.id_validator.validate_extracted_var_from_accessor(
                                variable_name,
                                value,
                                rule_name,
                            )?;
                            self.id_validator.validate_extracted_var_from_accessor(
                                variable_rule,
                                value,
                                rule_name,
                            )?;
                            Ok(Accessor::ExtractedVar {
                                rule_name: variable_rule.to_owned(),
                                key: variable_name.to_owned(),
                            })
                        } else {
                            Err(MatcherError::NotValidIdOrNameError {
                                message: format!(
                                    "Invalid extracted variables accessor [{}] for rule [{}]",
                                    value, rule_name
                                ),
                            })
                        }
                    }
                    _ => Err(MatcherError::UnknownAccessorError { accessor: value.to_owned() }),
                }
            }
            _value => Ok(Accessor::Constant { value: Value::Text(input.to_owned()) }),
        };

        trace!(
            "AccessorBuilder - build: return accessor [{:?}] for input value [{}]",
            &result,
            input
        );
        result
    }
}

/// An Accessor returns the value of a specific field of an Event.
/// The following Accessors are defined:
/// - Constant: returns a constant value regardless of the Event;
/// - CreatedTs: returns the value of the "created_ms" field of an Event
/// - ExtractedVar: returns the value of one extracted variable
/// - Payload: returns the value of an entry in the payload of an Event
/// - Type: returns the value of the "type" field of an Event
/// - Event: returns the entire Event
#[derive(PartialEq, Debug)]
pub enum Accessor {
    Constant { value: Value },
    CreatedMs,
    ExtractedVar { rule_name: String, key: String },
    Payload { parser: Parser },
    Type,
    Event,
}

impl Accessor {
    pub fn get<'o>(
        &'o self,
        event: &'o InternalEvent,
        extracted_vars: Option<&'o HashMap<String, Value>>,
    ) -> Option<Cow<'o, Value>> {
        match &self {
            Accessor::Constant { value } => Some(Cow::Borrowed(&value)),
            Accessor::CreatedMs => Some(Cow::Borrowed(&event.created_ms)),
            Accessor::ExtractedVar { rule_name, key } => extracted_vars
                .and_then(|vars| vars.get(rule_name.as_str()))
                .and_then(|vars| {
                    match vars {
                        Value::Map(payload) => payload.get(key.as_str()),
                        _ => None
                    }
                })
                .map(|value| Cow::Borrowed(value)),
            Accessor::Payload { parser } => parser.parse_value(&event.payload),
            Accessor::Type => Some(Cow::Borrowed(&event.event_type)),
            Accessor::Event => {
                let event_value: Value = event.clone().into();
                Some(Cow::Owned(event_value))
            }
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;
    use tornado_common_api::*;

    #[test]
    fn should_return_a_constant_value() {
        let accessor = Accessor::Constant { value: Value::Text("constant_value".to_owned()) };

        let event = InternalEvent::new(Event::new("event_type_string"));

        let result = accessor.get(&event, None).unwrap();

        assert_eq!("constant_value", result.as_ref());
    }

    #[test]
    fn should_not_trigger_a_constant_value() {
        let accessor = Accessor::Constant { value: Value::Text("  constant_value  ".to_owned()) };

        let event = InternalEvent::new(Event::new("event_type_string"));

        let result = accessor.get(&event, None).unwrap();

        assert_eq!("  constant_value  ", result.as_ref());
    }

    #[test]
    fn should_return_the_event_type() {
        let accessor = Accessor::Type {};

        let event = InternalEvent::new(Event::new("event_type_string"));

        let result = accessor.get(&event, None).unwrap();

        assert_eq!("event_type_string", result.as_ref());
    }

    #[test]
    fn should_return_the_event_created_ms() {
        let accessor = Accessor::CreatedMs {};

        let event = InternalEvent::new(Event::new("event_type_string"));

        let result = accessor.get(&event, None);

        let created_ms = result.unwrap().get_number().unwrap().clone();
        assert!(created_ms.is_u64());
        assert!(created_ms.as_u64().unwrap() > 0);
    }

    #[test]
    fn should_return_value_from_payload_if_exists() {
        let accessor = Accessor::Payload { parser: Parser::build_parser("${body}").unwrap() };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));

        let result = accessor.get(&event, None).unwrap();

        assert_eq!("body_value", result.as_ref());
    }

    #[test]
    fn should_return_bool_value_from_payload() {
        // Arrange
        let accessor = Accessor::Payload { parser: Parser::build_parser("${bool_true}").unwrap() };

        let mut payload = HashMap::new();
        payload.insert("bool_true".to_owned(), Value::Bool(true));
        payload.insert("bool_false".to_owned(), Value::Bool(false));

        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));

        // Act
        let result = accessor.get(&event, None).unwrap();

        // Assert
        assert_eq!(&true, result.as_ref());
    }

    #[test]
    fn should_return_number_value_from_payload() {
        // Arrange
        let accessor = Accessor::Payload { parser: Parser::build_parser("${num_555}").unwrap() };

        let mut payload = HashMap::new();
        payload.insert("num_555".to_owned(), Value::Number(Number::Float(555.0)));

        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));

        // Act
        let result = accessor.get(&event, None).unwrap();

        // Assert
        assert_eq!(555.0, result.as_ref().get_number().unwrap().as_f64());
    }

    #[test]
    fn should_return_non_text_nodes() {
        // Arrange
        let accessor = Accessor::Payload { parser: Parser::build_parser("${body}").unwrap() };

        let mut body_payload = HashMap::new();
        body_payload.insert("first".to_owned(), Value::Text("body_first_value".to_owned()));
        body_payload.insert("second".to_owned(), Value::Text("body_second_value".to_owned()));

        let body_clone = body_payload.clone();

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Map(body_payload));

        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));

        // Act
        let result = accessor.get(&event, None).unwrap();

        // Assert
        assert_eq!(&Value::Map(body_clone), result.as_ref());
    }

    #[test]
    fn should_return_value_from_nested_map_if_exists() {
        // Arrange
        let accessor = Accessor::Payload { parser: Parser::build_parser("${body.first}").unwrap() };

        let mut body_payload = HashMap::new();
        body_payload.insert("first".to_owned(), Value::Text("body_first_value".to_owned()));
        body_payload.insert("second".to_owned(), Value::Text("body_second_value".to_owned()));

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Map(body_payload));

        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));

        // Act
        let result = accessor.get(&event, None).unwrap();

        // Assert
        assert_eq!("body_first_value", result.as_ref());
    }

    #[test]
    fn should_return_value_from_nested_array_if_exists() {
        // Arrange
        let accessor = Accessor::Payload { parser: Parser::build_parser("${body[1]}").unwrap() };

        let mut payload = HashMap::new();
        payload.insert(
            "body".to_owned(),
            Value::Array(vec![
                Value::Text("body_first_value".to_owned()),
                Value::Text("body_second_value".to_owned()),
            ]),
        );

        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));

        // Act
        let result = accessor.get(&event, None).unwrap();

        // Assert
        assert_eq!("body_second_value", result.as_ref());
    }

    #[test]
    fn should_accept_double_quotas_delimited_keys() {
        // Arrange
        let accessor = Accessor::Payload {
            parser: Parser::build_parser(r#"${body."second.with.dot"}"#).unwrap(),
        };

        let mut body_payload = HashMap::new();
        body_payload.insert("first".to_owned(), Value::Text("body_first_value".to_owned()));
        body_payload
            .insert("second.with.dot".to_owned(), Value::Text("body_second_value".to_owned()));

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Map(body_payload));

        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));

        // Act
        let result = accessor.get(&event, None).unwrap();

        // Assert
        assert_eq!("body_second_value", result.as_ref());
    }

    #[test]
    fn should_return_none_from_payload_if_not_exists() {
        let accessor = Accessor::Payload { parser: Parser::build_parser("${date}").unwrap() };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));
        let result = accessor.get(&event, None);

        assert!(result.is_none());
    }

    #[test]
    fn should_return_the_entire_event() {
        let accessor = Accessor::Event {};

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));
        let result = accessor.get(&event, None).unwrap();

        let event_value: Value = event.clone().into();
        assert_eq!(&event_value, result.as_ref());
    }

    #[test]
    fn should_return_the_entire_payload() {
        let accessor = Accessor::Payload { parser: Parser::build_parser("${}").unwrap() };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));
        let result = accessor.get(&event, None).unwrap();

        assert_eq!(&event.payload, result.as_ref());
    }

    #[test]
    fn should_return_value_from_extracted_var() {
        let accessor =
            Accessor::ExtractedVar { rule_name: "rule1".to_owned(), key: "body".to_owned() };

        let event = InternalEvent::new(Event::new("event_type_string"));
        let mut extracted_vars_inner = HashMap::new();
        extracted_vars_inner.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        extracted_vars_inner.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let mut extracted_vars = HashMap::new();
        extracted_vars.insert("rule1".to_owned(), Value::Map(extracted_vars_inner));

        let result = accessor.get(&event, Some(&extracted_vars)).unwrap();

        assert_eq!("body_value", result.as_ref());
    }

    #[test]
    fn should_return_value_from_extracted_var_of_current_rule() {
        let builder = AccessorBuilder::new();
        let value = "${_variables.body}".to_owned();

        let accessor = builder.build("current_rule_name", &value).unwrap();

        let event = InternalEvent::new(Event::new("event_type_string"));
        let mut extracted_vars_current = HashMap::new();
        extracted_vars_current.insert("body".to_owned(), Value::Text("current_body".to_owned()));
        extracted_vars_current
            .insert("subject".to_owned(), Value::Text("current_subject".to_owned()));

        let mut extracted_vars_custom = HashMap::new();
        extracted_vars_custom.insert("body".to_owned(), Value::Text("custom_body".to_owned()));
        extracted_vars_custom
            .insert("subject".to_owned(), Value::Text("custom_subject".to_owned()));

        let mut extracted_vars = HashMap::new();
        extracted_vars.insert("current_rule_name".to_owned(), Value::Map(extracted_vars_current));
        extracted_vars.insert("custom_rule_name".to_owned(), Value::Map(extracted_vars_custom));

        let result = accessor.get(&event, Some(&extracted_vars)).unwrap();

        assert_eq!("current_body", result.as_ref());
    }

    #[test]
    fn should_return_value_from_extracted_var_of_custom_rule() {
        let builder = AccessorBuilder::new();
        let value = "${_variables.custom_rule_name.body}".to_owned();

        let accessor = builder.build("current_rule_name", &value).unwrap();

        let event = InternalEvent::new(Event::new("event_type_string"));
        let mut extracted_vars_current = HashMap::new();
        extracted_vars_current.insert("body".to_owned(), Value::Text("current_body".to_owned()));
        extracted_vars_current
            .insert("subject".to_owned(), Value::Text("current_subject".to_owned()));

        let mut extracted_vars_custom = HashMap::new();
        extracted_vars_custom.insert("body".to_owned(), Value::Text("custom_body".to_owned()));
        extracted_vars_custom
            .insert("subject".to_owned(), Value::Text("custom_subject".to_owned()));

        let mut extracted_vars = HashMap::new();
        extracted_vars.insert("current_rule_name".to_owned(), Value::Map(extracted_vars_current));
        extracted_vars.insert("custom_rule_name".to_owned(), Value::Map(extracted_vars_custom));

        let result = accessor.get(&event, Some(&extracted_vars)).unwrap();

        assert_eq!("custom_body", result.as_ref());
    }

    #[test]
    fn should_return_none_if_no_match() {
        let accessor =
            Accessor::ExtractedVar { rule_name: "rule1".to_owned(), key: "body".to_owned() };

        let event = InternalEvent::new(Event::new("event_type_string"));

        let result = accessor.get(&event, None);

        assert!(result.is_none());
    }

    #[test]
    fn builder_should_return_constant_accessor() {
        let builder = AccessorBuilder::new();
        let value = "constant_value".to_owned();

        let accessor = builder.build("", &value).unwrap();

        assert_eq!(Accessor::Constant { value: Value::Text(value) }, accessor);
    }

    #[test]
    fn builder_should_return_event_accessor_for_type() {
        let builder = AccessorBuilder::new();
        let value = "${event.type}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        assert_eq!(Accessor::Type {}, accessor)
    }

    #[test]
    fn builder_should_return_event_accessor_for_created_ms() {
        let builder = AccessorBuilder::new();
        let value = "${event.created_ms}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        assert_eq!(Accessor::CreatedMs {}, accessor)
    }

    #[test]
    fn builder_should_return_payload_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        assert_eq!(Accessor::Payload { parser: Parser::build_parser("${}").unwrap() }, accessor)
    }

    #[test]
    fn builder_should_return_payload_with_inner_key() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload.key}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        assert_eq!(Accessor::Payload { parser: Parser::build_parser("${key}").unwrap() }, accessor)
    }

    #[test]
    fn builder_should_return_payload_accessor_with_nested_keys() {
        let builder = AccessorBuilder::new();
        let value = r#"${event.payload.first.second."th. ird"."four"}"#.to_owned();

        let accessor = builder.build("", &value).unwrap();

        assert_eq!(
            Accessor::Payload {
                parser: Parser::build_parser(r#"${first.second."th. ird"."four"}"#).unwrap()
            },
            accessor
        )
    }

    #[test]
    fn builder_should_return_current_rule_extracted_var_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${_variables.key}".to_owned();

        let accessor = builder.build("current_rule_name", &value).unwrap();

        assert_eq!(
            Accessor::ExtractedVar {
                rule_name: "current_rule_name".to_owned(),
                key: "key".to_owned()
            },
            accessor
        )
    }

    #[test]
    fn builder_should_return_custom_rule_extracted_var_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${_variables.custom_rule.key}".to_owned();

        let accessor = builder.build("current_rule_name", &value).unwrap();

        assert_eq!(
            Accessor::ExtractedVar { rule_name: "custom_rule".to_owned(), key: "key".to_owned() },
            accessor
        )
    }

    #[test]
    fn builder_should_return_event_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));
        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));

        let result = accessor.get(&event, None).unwrap();

        let event_value: Value = event.clone().into();
        assert_eq!(&event_value, result.as_ref());
        assert_eq!(Accessor::Event {}, accessor)
    }

    #[test]
    fn builder_should_return_payload_accessor_with_expected_key() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload.body}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = InternalEvent::new(Event::new_with_payload("event_type_string", payload));

        let result = accessor.get(&event, None).unwrap();

        assert_eq!("body_value", result.as_ref());
    }

    #[test]
    fn builder_should_return_error_if_unknown_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${events.types}".to_owned();

        let accessor = builder.build("", &value);

        assert!(&accessor.is_err());

        match accessor.err().unwrap() {
            MatcherError::UnknownAccessorError { accessor } => assert_eq!(value, accessor),
            _ => assert!(false),
        };
    }

    #[test]
    fn builder_should_return_error_if_unknown_accessor_with_event_suffix() {
        let builder = AccessorBuilder::new();
        let value = "${events}".to_owned();

        let accessor = builder.build("", &value);

        assert!(&accessor.is_err());

        match accessor.err().unwrap() {
            MatcherError::UnknownAccessorError { accessor } => assert_eq!(value, accessor),
            _ => assert!(false),
        };
    }

    #[test]
    fn builder_should_return_error_if_unknown_accessor_with_payload_suffix() {
        let builder = AccessorBuilder::new();
        let value = "${event.payloads}".to_owned();

        let accessor = builder.build("", &value);

        assert!(&accessor.is_err());

        match accessor.err().unwrap() {
            MatcherError::UnknownAccessorError { accessor } => assert_eq!(value, accessor),
            _ => assert!(false),
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
    fn builder_should_return_error_if_wrong_extracted_var_name() {
        let builder = AccessorBuilder::new();
        let value = "${_variables.not.valid.at.all}";

        let accessor = builder.build("", value);

        assert!(&accessor.is_err());

        match accessor.err().unwrap() {
            MatcherError::NotValidIdOrNameError { message } => {
                assert!(message.contains("${_variables.not.valid.at.all}"));
            }
            _ => assert!(false),
        };
    }
}
