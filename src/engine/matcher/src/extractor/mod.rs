use chrono::prelude::Local;
use std::collections::HashMap;
use tornado_common::Event;

/// An Extractor returns the value of a field of an Event
pub trait Extractor {
    fn get(&self, event: &Event) -> Option<String>;
}

/// Returns a constant value regardless of the Event
pub struct ConstantExtractor {
    value: String
}

impl Extractor for ConstantExtractor {
    fn get(&self, _event: &Event) -> Option<String> {
        return Some(self.value.to_owned());
    }
}

/// Returns the type of an event
pub struct TypeExtractor {}

impl Extractor for TypeExtractor {
    fn get(&self, event: &Event) -> Option<String> {
        return Some(event.event_type.to_owned());
    }
}


/// Returns the created_ts of an event
pub struct CreatedTsExtractor {}

impl Extractor for CreatedTsExtractor {
    fn get(&self, event: &Event) -> Option<String> {
        return Some(format!("{}", event.created_ts));
    }
}

/// Returns a value from the payload of an event
pub struct PayloadExtractor {
    key: String
}

impl Extractor for PayloadExtractor {
    fn get(&self, event: &Event) -> Option<String> {
        return event.payload.get(&self.key).cloned();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_return_a_constant_value() {

        let extractor = ConstantExtractor{
            value: "constant_value".to_owned()
        };

        let result = extractor.get(&Event{
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload: HashMap::new()
        });

        assert_eq!("constant_value".to_owned(), result.unwrap());
    }

    #[test]
    fn should_return_the_event_type() {

        let extractor = TypeExtractor{};

        let result = extractor.get(&Event{
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload: HashMap::new()
        });

        assert_eq!("event_type_string".to_owned(), result.unwrap());
    }

    #[test]
    fn should_return_the_event_created_ts() {

        let extractor = CreatedTsExtractor{};

        let dt = Local::now();
        let created_ts= dt.timestamp_millis() as u64;

        let result = extractor.get(&Event{
            created_ts,
            event_type: "event_type_string".to_owned(),
            payload: HashMap::new()
        });

        assert_eq!(format!("{}", created_ts), result.unwrap());
    }

    #[test]
    fn should_return_value_from_payload_if_exists() {

        let extractor = PayloadExtractor{
            key: "body".to_owned()
        };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), "body_value".to_owned());
        payload.insert("subject".to_owned(), "subject_value".to_owned());

        let result = extractor.get(&Event{
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload
        });

        assert_eq!("body_value".to_owned(), result.unwrap());
    }

    #[test]
    fn should_return_none_from_payload_if_not_exists() {

        let extractor = PayloadExtractor{
            key: "date".to_owned()
        };

        let mut payload = HashMap::new();
        payload.insert("body".to_owned(), "body_value".to_owned());
        payload.insert("subject".to_owned(), "subject_value".to_owned());

        let result = extractor.get(&Event{
            created_ts: 0,
            event_type: "event_type_string".to_owned(),
            payload
        });

        assert!(result.is_none());
    }
}