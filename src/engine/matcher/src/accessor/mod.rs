use tornado_common::Event;

#[derive(Fail, Debug)]
pub enum AccessorBuilderError {
    #[fail(
        display = "UnknownAccessorError: Unknown accessor: [{}]",
        accessor
    )]
    UnknownAccessorError { accessor: String },
    #[fail(display = "WrongPayloadKeyError: [{}]", payload_key)]
    WrongPayloadKeyError { payload_key: String },
}

pub struct AccessorBuilder {
    start_delimiter: &'static str,
    end_delimiter: &'static str,
}

const EVENT_TYPE_KEY: &str = "event.type";
const EVENT_CREATED_TS_KEY: &str = "event.created_ts";
const EVENT_PAYLOAD_SUFFIX: &str = "event.payload.";

impl AccessorBuilder {
    pub fn new() -> AccessorBuilder {
        AccessorBuilder {
            start_delimiter: "${",
            end_delimiter: "}",
        }
    }

    pub fn build(&self, value: &String) -> Result<Box<Accessor>, AccessorBuilderError> {
        match value.trim() {
            value
                if value.starts_with(self.start_delimiter)
                    && value.ends_with(self.end_delimiter) =>
            {
                let path =
                    &value[self.start_delimiter.len()..(value.len() - self.end_delimiter.len())];
                match path.trim() {
                    EVENT_TYPE_KEY => Ok(Box::new(TypeAccessor {})),
                    EVENT_CREATED_TS_KEY => Ok(Box::new(CreatedTsAccessor {})),
                    val if val.starts_with(EVENT_PAYLOAD_SUFFIX) => {
                        let key = &val[EVENT_PAYLOAD_SUFFIX.len()..];
                        if key.is_empty() {
                            return Err(AccessorBuilderError::WrongPayloadKeyError {
                                payload_key: path.to_owned(),
                            });
                        }
                        return Ok(Box::new(PayloadAccessor {
                            key: key.to_owned(),
                        }));
                    }
                    _ => Err(AccessorBuilderError::UnknownAccessorError {
                        accessor: value.to_owned(),
                    }),
                }
            }
            value => Ok(Box::new(ConstantAccessor {
                value: value.to_owned(),
            })),
        }
    }
}

/// An Accessor returns the value of a field of an Event
pub trait Accessor {
    fn name(&self) -> &str;
    fn get(&self, event: &Event) -> Option<String>;
}

/// Returns a constant value regardless of the Event
pub struct ConstantAccessor {
    value: String,
}

impl Accessor for ConstantAccessor {
    fn name(&self) -> &str {
        "constant"
    }

    fn get(&self, _event: &Event) -> Option<String> {
        return Some(self.value.to_owned());
    }
}

/// Returns the type of an event
pub struct TypeAccessor {}

impl Accessor for TypeAccessor {
    fn name(&self) -> &str {
        "type"
    }

    fn get(&self, event: &Event) -> Option<String> {
        return Some(event.event_type.to_owned());
    }
}

/// Returns the created_ts of an event
pub struct CreatedTsAccessor {}

impl Accessor for CreatedTsAccessor {
    fn name(&self) -> &str {
        "created_ts"
    }

    fn get(&self, event: &Event) -> Option<String> {
        return Some(format!("{}", event.created_ts));
    }
}

/// Returns a value from the payload of an event
pub struct PayloadAccessor {
    key: String,
}

impl Accessor for PayloadAccessor {
    fn name(&self) -> &str {
        "payload"
    }

    fn get(&self, event: &Event) -> Option<String> {
        return event.payload.get(&self.key).cloned();
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use chrono::prelude::Local;
    use std::collections::HashMap;

    #[test]
    fn should_return_a_constant_value() {
        let accessor = ConstantAccessor {
            value: "constant_value".to_owned(),
        };

        let result = accessor.get(&Event {
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload: HashMap::new(),
        });

        assert_eq!("constant_value".to_owned(), result.unwrap());
    }

    #[test]
    fn should_return_the_event_type() {
        let accessor = TypeAccessor {};

        let result = accessor.get(&Event {
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload: HashMap::new(),
        });

        assert_eq!("event_type_string".to_owned(), result.unwrap());
    }

    #[test]
    fn should_return_the_event_created_ts() {
        let accessor = CreatedTsAccessor {};

        let dt = Local::now();
        let created_ts = dt.timestamp_millis() as u64;

        let result = accessor.get(&Event {
            created_ts,
            event_type: "event_type_string".to_owned(),
            payload: HashMap::new(),
        });

        assert_eq!(format!("{}", created_ts), result.unwrap());
    }

    #[test]
    fn should_return_value_from_payload_if_exists() {
        let accessor = PayloadAccessor {
            key: "body".to_owned(),
        };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), "body_value".to_owned());
        payload.insert("subject".to_owned(), "subject_value".to_owned());

        let result = accessor.get(&Event {
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload,
        });

        assert_eq!("body_value".to_owned(), result.unwrap());
    }

    #[test]
    fn should_return_none_from_payload_if_not_exists() {
        let accessor = PayloadAccessor {
            key: "date".to_owned(),
        };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), "body_value".to_owned());
        payload.insert("subject".to_owned(), "subject_value".to_owned());

        let result = accessor.get(&Event {
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload,
        });

        assert!(result.is_none());
    }

    #[test]
    fn builder_should_return_constant_accessor() {
        let builder = AccessorBuilder::new();
        let value = "constant_value".to_owned();

        let accessor = builder.build(&value).unwrap();

        assert_eq!("constant", accessor.name())
    }

    #[test]
    fn builder_should_return_type_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event.type}".to_owned();

        let accessor = builder.build(&value).unwrap();

        assert_eq!("type", accessor.name())
    }

    #[test]
    fn builder_should_return_created_ts_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event.created_ts}".to_owned();

        let accessor = builder.build(&value).unwrap();

        assert_eq!("created_ts", accessor.name())
    }

    #[test]
    fn builder_should_return_payload_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload.key}".to_owned();

        let accessor = builder.build(&value).unwrap();

        assert_eq!("payload", accessor.name())
    }

    #[test]
    fn builder_should_return_payload_accessor_with_expected_key() {
        let builder = AccessorBuilder::new();
        let value = "${event.payload.body}".to_owned();

        let accessor = builder.build(&value).unwrap();

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), "body_value".to_owned());
        payload.insert("subject".to_owned(), "subject_value".to_owned());

        let result = accessor.get(&Event {
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload,
        });

        assert_eq!("body_value".to_owned(), result.unwrap());
    }

    #[test]
    fn builder_should_return_error_if_unknown_accessor() {
        let builder = AccessorBuilder::new();
        let value = "${event.types}".to_owned();

        let accessor = builder.build(&value);

        assert!(&accessor.is_err());

        match accessor.err().unwrap() {
            AccessorBuilderError::UnknownAccessorError { accessor } => {
                assert_eq!(value, accessor)
            }
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
            AccessorBuilderError::WrongPayloadKeyError { payload_key } => {
                assert_eq!("event.payload.".to_owned(), payload_key)
            }
            _ => assert!(false),
        };
    }
}
