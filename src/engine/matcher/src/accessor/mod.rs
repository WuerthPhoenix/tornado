use error::MatcherError;
use std::borrow::Cow;
use tornado_common_api::Event;

#[derive(Default)]
pub struct AccessorBuilder {
    start_delimiter: &'static str,
    end_delimiter: &'static str,
}

const EVENT_TYPE_KEY: &str = "event.type";
const EVENT_CREATED_TS_KEY: &str = "event.created_ts";
const EVENT_PAYLOAD_SUFFIX: &str = "event.payload.";

/// A builder for the Event Accessors
impl AccessorBuilder {
    pub fn new() -> AccessorBuilder {
        AccessorBuilder {
            start_delimiter: "${",
            end_delimiter: "}",
        }
    }

    /// Returns an Accessor instance based on its string definition.
    /// E.g.:
    /// - "${event.type}" -> returns an instance of Accessor::Type
    /// - "${event.created_ts}" -> returns an instance of Accessor::CreatedTs
    /// - "${event.payload.body}" -> returns an instance of Accessor::Payload that returns the value of the entry with key "body" from the event payload
    /// - "event.type" -> returns an instance of Accessor::Constant that always return the String "event.type"
    pub fn build(&self, value: &str) -> Result<Accessor, MatcherError> {
        match value.trim() {
            value
                if value.starts_with(self.start_delimiter)
                    && value.ends_with(self.end_delimiter) =>
            {
                let path =
                    &value[self.start_delimiter.len()..(value.len() - self.end_delimiter.len())];
                match path.trim() {
                    EVENT_TYPE_KEY => Ok(Accessor::Type {}),
                    EVENT_CREATED_TS_KEY => Ok(Accessor::CreatedTs {}),
                    val if val.starts_with(EVENT_PAYLOAD_SUFFIX) => {
                        let key = &val[EVENT_PAYLOAD_SUFFIX.len()..];
                        if key.is_empty() {
                            return Err(MatcherError::AccessorWrongPayloadKeyError {
                                payload_key: path.to_owned(),
                            });
                        }
                        Ok(Accessor::Payload {
                            key: key.to_owned(),
                        })
                    }
                    _ => Err(MatcherError::UnknownAccessorError {
                        accessor: value.to_owned(),
                    }),
                }
            }
            value => Ok(Accessor::Constant {
                value: value.to_owned(),
            }),
        }
    }
}

/// An Accessor returns the value of a specific field of an Event.
/// The following Accessors are defined:
/// - Constant : returns a constant value regardless of the Event;
/// - CreatedTs : returns the value of the "created_ts" field of an Event
/// - Payload : returns the value of an entry in the payload of an Event
/// - Type : returns the value of the "type" field of an Event
#[derive(PartialEq, Debug)]
pub enum Accessor {
    Constant { value: String },
    Type {},
    CreatedTs {},
    Payload { key: String },
}

impl Accessor {
    pub fn get<'o>(&'o self, event: &'o Event) -> Option<Cow<'o, str>> {
        match &self {
            Accessor::Constant { value } => Some(value.into()),
            Accessor::CreatedTs {} => Some(format!("{}", event.created_ts).into()),
            Accessor::Payload { key } => event
                .payload
                .get(key)
                .map(|value| value.as_str().into()),
            Accessor::Type {} => Some((&event.event_type).into()),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use chrono::prelude::Local;
    use std::collections::HashMap;

    #[test]
    fn should_return_a_constant_value() {
        let accessor = Accessor::Constant {
            value: "constant_value".to_owned(),
        };

        let event = Event {
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload: HashMap::new(),
        };

        let result = accessor.get(&event).unwrap();

        assert_eq!("constant_value", result);

        match result {
            Cow::Borrowed(_) => assert!(true),
            _ => assert!(false)
        }
    }

    #[test]
    fn should_return_the_event_type() {
        let accessor = Accessor::Type {};

        let event = Event {
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload: HashMap::new(),
        };

        let result = accessor.get(&event).unwrap();

        assert_eq!("event_type_string", result);

        match result {
            Cow::Borrowed(_) => assert!(true),
            _ => assert!(false)
        }

    }

    #[test]
    fn should_return_the_event_created_ts() {
        let accessor = Accessor::CreatedTs {};

        let dt = Local::now();
        let created_ts = dt.timestamp_millis() as u64;

        let event = Event {
            created_ts,
            event_type: "event_type_string".to_owned(),
            payload: HashMap::new(),
        };

        let result = accessor.get(&event).unwrap();

        assert_eq!(format!("{}", created_ts).as_str(), result);

        match result {
            Cow::Owned(_) => assert!(true),
            _ => assert!(false)
        }
    }

    #[test]
    fn should_return_value_from_payload_if_exists() {
        let accessor = Accessor::Payload {
            key: "body".to_owned(),
        };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), "body_value".to_owned());
        payload.insert("subject".to_owned(), "subject_value".to_owned());

        let event = Event {
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload,
        };
        let result = accessor.get(&event).unwrap();

        assert_eq!("body_value", result);

        match result {
            Cow::Borrowed(_) => assert!(true),
            _ => assert!(false)
        }
    }

    #[test]
    fn should_return_none_from_payload_if_not_exists() {
        let accessor = Accessor::Payload {
            key: "date".to_owned(),
        };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), "body_value".to_owned());
        payload.insert("subject".to_owned(), "subject_value".to_owned());

        let event = Event {
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload,
        };
        let result = accessor.get(&event);

        assert!(result.is_none());
    }

    #[test]
    fn builder_should_return_constant_accessor() {
        let builder = AccessorBuilder::new();
        let value = "constant_value".to_owned();

        let accessor = builder.build(&value).unwrap();

        assert_eq!(Accessor::Constant { value }, accessor)
    }

    #[test]
    fn builder_should_return_type_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event.type}".to_owned();

        let accessor = builder.build(&value).unwrap();

        assert_eq!(Accessor::Type {}, accessor)
    }

    #[test]
    fn builder_should_return_created_ts_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event.created_ts}".to_owned();

        let accessor = builder.build(&value).unwrap();

        assert_eq!(Accessor::CreatedTs {}, accessor)
    }

    #[test]
    fn builder_should_return_payload_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload.key}".to_owned();

        let accessor = builder.build(&value).unwrap();

        assert_eq!(
            Accessor::Payload {
                key: "key".to_owned()
            },
            accessor
        )
    }

    #[test]
    fn builder_should_return_payload_accessor_with_expected_key() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload.body}".to_owned();

        let accessor = builder.build(&value).unwrap();

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), "body_value".to_owned());
        payload.insert("subject".to_owned(), "subject_value".to_owned());

        let event = Event {
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload,
        };
        let result = accessor.get(&event);

        assert_eq!("body_value", result.unwrap());
    }

    #[test]
    fn builder_should_return_error_if_unknown_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event.types}".to_owned();

        let accessor = builder.build(&value);

        assert!(&accessor.is_err());

        match accessor.err().unwrap() {
            MatcherError::UnknownAccessorError { accessor } => assert_eq!(value, accessor),
            _ => assert!(false),
        };
    }

    #[test]
    fn builder_should_return_error_if_wrong_payload() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload.}".to_owned();

        let accessor = builder.build(&value);

        assert!(&accessor.is_err());

        match accessor.err().unwrap() {
            MatcherError::AccessorWrongPayloadKeyError { payload_key } => {
                assert_eq!("event.payload.".to_owned(), payload_key)
            }
            _ => assert!(false),
        };
    }

}
