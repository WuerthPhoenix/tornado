use crate::error::MatcherError;
use crate::model::ProcessedEvent;
use crate::validator::id::IdValidator;
use log::*;
use regex::Regex as RustRegex;
use std::borrow::Cow;
use tornado_common_api::Value;

pub struct AccessorBuilder {
    id_validator: IdValidator,
    start_delimiter: &'static str,
    end_delimiter: &'static str,
    regex: RustRegex,
}

impl Default for AccessorBuilder {
    fn default() -> Self {
        AccessorBuilder {
            id_validator: IdValidator::new(),
            start_delimiter: "${",
            end_delimiter: "}",
            regex: RustRegex::new(PAYLOAD_KEY_PARSE_REGEX)
                .expect("AccessorBuilder regex should be valid"),
        }
    }
}

const CURRENT_RULE_EXTRACTED_VAR_SUFFIX: &str = "_variables.";
const EVENT_KEY: &str = "event";
const EVENT_TYPE_KEY: &str = "event.type";
const EVENT_CREATED_TS_KEY: &str = "event.created_ts";
const EVENT_PAYLOAD_SUFFIX: &str = "event.payload";
const PAYLOAD_KEY_PARSE_REGEX: &str = r#"("[^"]+"|[^\.^\[]+|\[[^\]]+\])"#;
const PAYLOAD_MAP_KEY_PARSE_TRAILING_DELIMITER: char = '"';
const PAYLOAD_ARRAY_KEY_START_DELIMITER: char = '[';
const PAYLOAD_ARRAY_KEY_END_DELIMITER: char = ']';

/// A builder for the Event Accessors
impl AccessorBuilder {
    pub fn new() -> AccessorBuilder {
        Default::default()
    }

    /// Returns an Accessor instance based on its string definition.
    /// E.g.:
    /// - "${event}": returns the entire Event instance
    /// - "${event.type}": returns an instance of Accessor::Type
    /// - "${event.created_ts}": returns an instance of Accessor::CreatedTs
    /// - "${event.payload}": returns the entire Payload of the Event
    /// - "${event.payload.body}": returns an instance of Accessor::Payload that returns the value of the entry with the key "body" from the event payload
    /// - "event.type": returns an instance of Accessor::Constant that always return the String "event.type"
    pub fn build(&self, rule_name: &str, input: &str) -> Result<Accessor, MatcherError> {
        info!("AccessorBuilder - build: build accessor [{}] for rule [{}]", input, rule_name);
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
                    EVENT_CREATED_TS_KEY => Ok(Accessor::CreatedTs {}),
                    val if (val.starts_with(&format!("{}.", EVENT_PAYLOAD_SUFFIX))
                        || val.eq(EVENT_PAYLOAD_SUFFIX)) =>
                    {
                        let key = val[EVENT_PAYLOAD_SUFFIX.len()..].trim();
                        let keys = self.parse_payload_key(key, value, rule_name)?;
                        Ok(Accessor::Payload { keys })
                    }
                    val if val.starts_with(CURRENT_RULE_EXTRACTED_VAR_SUFFIX) => {
                        let key = val[CURRENT_RULE_EXTRACTED_VAR_SUFFIX.len()..].trim();
                        self.id_validator
                            .validate_extracted_var_from_accessor(key, value, rule_name)?;
                        Ok(Accessor::ExtractedVar { key: format!("{}.{}", rule_name, key) })
                    }
                    _ => Err(MatcherError::UnknownAccessorError { accessor: value.to_owned() }),
                }
            }
            _value => Ok(Accessor::Constant { value: Value::Text(input.to_owned()) }),
        };

        info!(
            "AccessorBuilder - build: return accessor [{:?}] for input value [{}]",
            &result, input
        );
        result
    }

    fn parse_payload_key(
        &self,
        key: &str,
        full_accessor: &str,
        rule_name: &str,
    ) -> Result<Vec<ValueGetter>, MatcherError> {
        self
            .regex
            .captures_iter(key)
            .map(|cap| {
                let capture = cap.get(0)
                    .ok_or_else(|| MatcherError::NotValidIdOrNameError {message: format!(
                        "Error parsing payload key [{}] from accessor [{}] for rule [{}]",
                        key, full_accessor, rule_name
                    )})?;
                let mut result = capture.as_str().to_string();

                // Remove trailing delimiters
                {
                    if result.starts_with(PAYLOAD_MAP_KEY_PARSE_TRAILING_DELIMITER) &&
                        result.ends_with(PAYLOAD_MAP_KEY_PARSE_TRAILING_DELIMITER) {
                        result = result[1..(result.len() - 1)].to_string();
                    }
                    if result.starts_with(PAYLOAD_ARRAY_KEY_START_DELIMITER) &&
                        result.ends_with(PAYLOAD_ARRAY_KEY_END_DELIMITER) {
                        result = result[1..(result.len() - 1)].to_string();
                        let index = usize::from_str_radix(&result, 10)
                            .map_err(|err| MatcherError::ParseOperatorError { message: format!("Cannot parse value [{}] to number: {}", &result, err) })?;
                        return Ok(ValueGetter::Array {index})
                    }
                    if result.contains(PAYLOAD_MAP_KEY_PARSE_TRAILING_DELIMITER) {
                        let error_message = format!(
                            "Payload key [{}] from accessor [{}] for rule [{}] contains not valid characters: [{}]",
                            key, full_accessor, rule_name, PAYLOAD_MAP_KEY_PARSE_TRAILING_DELIMITER
                        );
                        return Err(MatcherError::NotValidIdOrNameError { message: error_message });
                    }
                }
                Ok(ValueGetter::Map {key: result})
            }).collect()
    }
}

/// An Accessor returns the value of a specific field of an Event.
/// The following Accessors are defined:
/// - Constant: returns a constant value regardless of the Event;
/// - CreatedTs: returns the value of the "created_ts" field of an Event
/// - ExtractedVar: returns the value of one extracted variable
/// - Payload: returns the value of an entry in the payload of an Event
/// - Type: returns the value of the "type" field of an Event
/// - Event: returns the entire Event
#[derive(PartialEq, Debug)]
pub enum Accessor {
    Constant { value: Value },
    CreatedTs {},
    ExtractedVar { key: String },
    Payload { keys: Vec<ValueGetter> },
    Type {},
    Event {},
}

impl Accessor {
    pub fn get<'o>(&'o self, event: &'o ProcessedEvent) -> Option<Cow<'o, Value>> {
        match &self {
            Accessor::Constant { value } => Some(Cow::Borrowed(&value)),
            Accessor::CreatedTs {} => Some(Cow::Borrowed(&event.event.created_ts)),
            Accessor::ExtractedVar { key } => {
                event.extracted_vars.get(key.as_str()).map(|value| Cow::Borrowed(value))
            }
            Accessor::Payload { keys } => {
                let mut value = Some(&event.event.payload);

                let mut count = 0;

                while count < keys.len() && value.is_some() {
                    value = value.and_then(|val| keys[count].get(val));
                    count += 1;
                }

                value.map(|value| Cow::Borrowed(value))
            }
            Accessor::Type {} => Some(Cow::Borrowed(&event.event.event_type)),
            Accessor::Event {} => {
                let event_value: Value = event.event.clone().into();
                Some(Cow::Owned(event_value))
            }
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ValueGetter {
    Map { key: String },
    Array { index: usize },
}

impl ValueGetter {
    pub fn get<'o>(&self, value: &'o Value) -> Option<&'o Value> {
        match self {
            ValueGetter::Map { key } => value.get_from_map(key),
            ValueGetter::Array { index } => value.get_from_array(*index),
        }
    }
}

impl Into<ValueGetter> for &str {
    fn into(self) -> ValueGetter {
        ValueGetter::Map { key: self.to_owned() }
    }
}

impl Into<ValueGetter> for usize {
    fn into(self) -> ValueGetter {
        ValueGetter::Array { index: self }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use chrono::prelude::DateTime;
    use std::collections::HashMap;
    use tornado_common_api::*;

    #[test]
    fn should_return_a_constant_value() {
        let accessor = Accessor::Constant { value: Value::Text("constant_value".to_owned()) };

        let event = ProcessedEvent::new(Event::new("event_type_string"));

        let result = accessor.get(&event).unwrap();

        assert_eq!("constant_value", result.as_ref());
    }

    #[test]
    fn should_not_trigger_a_constant_value() {
        let accessor = Accessor::Constant { value: Value::Text("  constant_value  ".to_owned()) };

        let event = ProcessedEvent::new(Event::new("event_type_string"));

        let result = accessor.get(&event).unwrap();

        assert_eq!("  constant_value  ", result.as_ref());
    }

    #[test]
    fn should_return_the_event_type() {
        let accessor = Accessor::Type {};

        let event = ProcessedEvent::new(Event::new("event_type_string"));

        let result = accessor.get(&event).unwrap();

        assert_eq!("event_type_string", result.as_ref());
    }

    #[test]
    fn should_return_the_event_created_ts() {
        let accessor = Accessor::CreatedTs {};

        let event = ProcessedEvent::new(Event::new("event_type_string"));

        let result = accessor.get(&event);

        assert!(DateTime::parse_from_rfc3339(to_option_str(&result).unwrap()).is_ok());
    }

    #[test]
    fn should_return_value_from_payload_if_exists() {
        let accessor = Accessor::Payload { keys: vec!["body".into()] };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = ProcessedEvent::new(Event::new_with_payload("event_type_string", payload));

        let result = accessor.get(&event).unwrap();

        assert_eq!("body_value", result.as_ref());
    }

    #[test]
    fn should_return_non_text_nodes() {
        // Arrange
        let accessor = Accessor::Payload { keys: vec!["body".into()] };

        let mut body_payload = HashMap::new();
        body_payload.insert("first".to_owned(), Value::Text("body_first_value".to_owned()));
        body_payload.insert("second".to_owned(), Value::Text("body_second_value".to_owned()));

        let body_clone = body_payload.clone();

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Map(body_payload));

        let event = ProcessedEvent::new(Event::new_with_payload("event_type_string", payload));

        // Act
        let result = accessor.get(&event).unwrap();

        // Assert
        assert_eq!(&Value::Map(body_clone), result.as_ref());
    }

    #[test]
    fn should_return_value_from_nested_map_if_exists() {
        // Arrange
        let accessor = Accessor::Payload { keys: vec!["body".into(), "first".into()] };

        let mut body_payload = HashMap::new();
        body_payload.insert("first".to_owned(), Value::Text("body_first_value".to_owned()));
        body_payload.insert("second".to_owned(), Value::Text("body_second_value".to_owned()));

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Map(body_payload));

        let event = ProcessedEvent::new(Event::new_with_payload("event_type_string", payload));

        // Act
        let result = accessor.get(&event).unwrap();

        // Assert
        assert_eq!("body_first_value", result.as_ref());
    }

    #[test]
    fn should_return_value_from_nested_array_if_exists() {
        // Arrange
        let accessor = Accessor::Payload { keys: vec!["body".into(), 1.into()] };

        let mut payload = HashMap::new();
        payload.insert(
            "body".to_owned(),
            Value::Array(vec![
                Value::Text("body_first_value".to_owned()),
                Value::Text("body_second_value".to_owned()),
            ]),
        );

        let event = ProcessedEvent::new(Event::new_with_payload("event_type_string", payload));

        // Act
        let result = accessor.get(&event).unwrap();

        // Assert
        assert_eq!("body_second_value", result.as_ref());
    }

    #[test]
    fn should_return_accept_double_quotas_delimited_keys() {
        // Arrange
        let accessor = Accessor::Payload { keys: vec!["body".into(), "second.with.dot".into()] };

        let mut body_payload = HashMap::new();
        body_payload.insert("first".to_owned(), Value::Text("body_first_value".to_owned()));
        body_payload
            .insert("second.with.dot".to_owned(), Value::Text("body_second_value".to_owned()));

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Map(body_payload));

        let event = ProcessedEvent::new(Event::new_with_payload("event_type_string", payload));

        // Act
        let result = accessor.get(&event).unwrap();

        // Assert
        assert_eq!("body_second_value", result.as_ref());
    }

    #[test]
    fn should_return_none_from_payload_if_not_exists() {
        let accessor = Accessor::Payload { keys: vec!["date".into()] };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = ProcessedEvent::new(Event::new_with_payload("event_type_string", payload));
        let result = accessor.get(&event);

        assert!(result.is_none());
    }

    #[test]
    fn should_return_the_entire_event() {
        let accessor = Accessor::Event {};

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = ProcessedEvent::new(Event::new_with_payload("event_type_string", payload));
        let result = accessor.get(&event).unwrap();

        let event_value: Value = event.event.clone().into();
        assert_eq!(&event_value, result.as_ref());
    }

    #[test]
    fn should_return_the_entire_payload() {
        let accessor = Accessor::Payload { keys: vec![] };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = ProcessedEvent::new(Event::new_with_payload("event_type_string", payload));
        let result = accessor.get(&event).unwrap();

        assert_eq!(&event.event.payload, result.as_ref());
    }

    #[test]
    fn should_return_value_from_extracted_var() {
        let accessor = Accessor::ExtractedVar { key: "rule1.body".to_owned() };

        let mut event = ProcessedEvent::new(Event::new("event_type_string"));

        event.extracted_vars.insert("rule1.body".to_owned(), Value::Text("body_value".to_owned()));
        event
            .extracted_vars
            .insert("rule1.subject".to_owned(), Value::Text("subject_value".to_owned()));

        let result = accessor.get(&event).unwrap();

        assert_eq!("body_value", result.as_ref());
    }

    #[test]
    fn should_return_none_if_no_match() {
        let accessor = Accessor::ExtractedVar { key: "rule1.body".to_owned() };

        let event = ProcessedEvent::new(Event::new("event_type_string"));

        let result = accessor.get(&event);

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
    fn builder_should_return_event_accessor_for_created_ts() {
        let builder = AccessorBuilder::new();
        let value = "${event.created_ts}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        assert_eq!(Accessor::CreatedTs {}, accessor)
    }

    #[test]
    fn builder_should_return_payload_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        assert_eq!(Accessor::Payload { keys: vec![] }, accessor)
    }

    #[test]
    fn builder_should_return_payload_with_inner_key() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload.key}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        assert_eq!(Accessor::Payload { keys: vec!["key".into()] }, accessor)
    }

    #[test]
    fn builder_should_return_payload_accessor_with_nested_keys() {
        let builder = AccessorBuilder::new();
        let value = r#"${event.payload.first.second."th. ird"."four"}"#.to_owned();

        let accessor = builder.build("", &value).unwrap();

        assert_eq!(
            Accessor::Payload {
                keys: vec!["first".into(), "second".into(), "th. ird".into(), "four".into()]
            },
            accessor
        )
    }

    #[test]
    fn builder_should_return_current_rule_extracted_var_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${_variables.key}".to_owned();

        let accessor = builder.build("current_rule_name", &value).unwrap();

        assert_eq!(Accessor::ExtractedVar { key: "current_rule_name.key".to_owned() }, accessor)
    }

    #[test]
    fn builder_should_return_event_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event}".to_owned();

        let accessor = builder.build("", &value).unwrap();

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));
        let event = ProcessedEvent::new(Event::new_with_payload("event_type_string", payload));

        let result = accessor.get(&event).unwrap();

        let event_value: Value = event.event.clone().into();
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

        let event = ProcessedEvent::new(Event::new_with_payload("event_type_string", payload));

        let result = accessor.get(&event).unwrap();

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
        let value = "${_variables.not.valid}";

        let accessor = builder.build("", value);

        assert!(&accessor.is_err());

        match accessor.err().unwrap() {
            MatcherError::NotValidIdOrNameError { message } => {
                assert!(message.contains("${_variables.not.valid}"));
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn builder_should_parse_a_payload_key() {
        let builder = AccessorBuilder::new();

        let expected: Vec<ValueGetter> = vec!["one".into()];
        assert_eq!(expected, builder.parse_payload_key("one", "", "").unwrap());

        let expected: Vec<ValueGetter> = vec!["one".into(), "two".into()];
        assert_eq!(expected, builder.parse_payload_key("one.two", "", "").unwrap());

        let expected: Vec<ValueGetter> = vec!["one".into(), "two".into()];
        assert_eq!(expected, builder.parse_payload_key("one.two.", "", "").unwrap());

        let expected: Vec<ValueGetter> = vec!["one".into(), "".into()];
        assert_eq!(expected, builder.parse_payload_key(r#"one."""#, "", "").unwrap());

        let expected: Vec<ValueGetter> = vec!["one".into(), "two".into(), "th ir.d".into()];
        assert_eq!(expected, builder.parse_payload_key(r#"one.two."th ir.d""#, "", "").unwrap());

        let expected: Vec<ValueGetter> =
            vec!["th ir.d".into(), "a".into(), "fourth".into(), "two".into()];
        assert_eq!(
            expected,
            builder.parse_payload_key(r#""th ir.d".a."fourth".two"#, "", "").unwrap()
        );

        let expected: Vec<ValueGetter> =
            vec!["payload".into(), "oids".into(), "SNMPv2-SMI::enterprises.14848.2.1.1.6.0".into()];
        assert_eq!(
            expected,
            builder
                .parse_payload_key(
                    r#"payload.oids."SNMPv2-SMI::enterprises.14848.2.1.1.6.0""#,
                    "",
                    ""
                )
                .unwrap()
        );
    }

    #[test]
    fn payload_key_parser_should_fail_if_key_contains_double_quotes() {
        // Arrange
        let builder = AccessorBuilder::new();

        // Act
        let result = builder.parse_payload_key(r#"o"ne"#, "", "");

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn payload_key_parser_should_fail_if_key_does_not_contain_both_trailing_and_ending_quotes() {
        // Arrange
        let builder = AccessorBuilder::new();

        // Act
        let result = builder.parse_payload_key(r#"one."two"#, "", "");

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn builder_parser_should_return_empty_vector_if_no_matches() {
        let builder = AccessorBuilder::new();
        let expected: Vec<ValueGetter> = vec![];
        assert_eq!(expected, builder.parse_payload_key("", "", "").unwrap())
    }

    #[test]
    fn builder_parser_should_return_empty_vector_if_single_dot() {
        let builder = AccessorBuilder::new();
        let expected: Vec<ValueGetter> = vec![];
        assert_eq!(expected, builder.parse_payload_key(".", "", "").unwrap())
    }

    #[test]
    fn builder_parser_should_return_ignore_trailing_dot() {
        let builder = AccessorBuilder::new();
        let expected: Vec<ValueGetter> = vec!["hello".into(), "world".into()];
        assert_eq!(expected, builder.parse_payload_key(".hello.world", "", "").unwrap())
    }

    #[test]
    fn builder_parser_should_not_return_array_reader_if_within_double_quotes() {
        let builder = AccessorBuilder::new();
        let expected: Vec<ValueGetter> =
            vec!["hello".into(), "world[11]".into(), "inner".into(), 0.into()];
        assert_eq!(
            expected,
            builder.parse_payload_key(r#"hello."world[11]".inner[0]"#, "", "").unwrap()
        )
    }

    #[test]
    fn builder_parser_should_return_array_reader() {
        let builder = AccessorBuilder::new();
        let expected: Vec<ValueGetter> =
            vec!["hello".into(), "world".into(), 11.into(), "inner".into(), 0.into()];
        assert_eq!(expected, builder.parse_payload_key("hello.world[11].inner[0]", "", "").unwrap())
    }
}
