extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate chrono;

use chrono::prelude::Local;
use std::borrow::Cow;
use std::collections::HashMap;

/// An Event is correlated with an incoming episode, incident, situation or any kind of message
///   that could be meaningful to the system.
/// Events are produced by Collectors and are sent to the Tornado Engine to be processed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub created_ts: String,
    pub payload: Payload,
}

impl Event {
    pub fn new<S: Into<String>>(event_type: S) -> Event {
        Event::new_with_payload(event_type, HashMap::new())
    }

    pub fn new_with_payload<S: Into<String>>(event_type: S, payload: Payload) -> Event {
        let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
        let created_ts: String = dt.to_rfc3339();
        Event { event_type: event_type.into(), created_ts, payload }
    }
}

impl Into<Value> for Event {
    fn into(self) -> Value {
        let mut payload = Payload::new();
        payload.insert("type".to_owned(), Value::Text(self.event_type));
        payload.insert("created_ts".to_owned(), Value::Text(self.created_ts));
        payload.insert("payload".to_owned(), Value::Map(self.payload));
        Value::Map(payload)
    }
}

/// An Action is produced when an Event matches a specific Rule.
/// Once created, the Tornado Engine sends the Action to the Executors to be resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub payload: Payload,
}

impl Action {
    pub fn new<S: Into<String>>(id: S) -> Action {
        Action::new_with_payload(id, HashMap::new())
    }
    pub fn new_with_payload<S: Into<String>>(id: S, payload: Payload) -> Action {
        Action { id: id.into(), payload }
    }
}

pub type Payload = HashMap<String, Value>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Value {
    Text(String),
    Array(Vec<Value>),
    Map(Payload),
}

impl Value {
    pub fn get_from_map(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(payload) => payload.get(key),
            Value::Array(_array) => None,
            Value::Text(_) => None,
        }
    }
    pub fn get_from_array(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Map(_payload) => None,
            Value::Array(array) => array.get(index),
            Value::Text(_) => None,
        }
    }
    pub fn text(&self) -> Option<&str> {
        match self {
            Value::Text(value) => Some(value),
            _ => None,
        }
    }
}

impl PartialEq<str> for Value {
    fn eq(&self, other: &str) -> bool {
        let option_text: Option<&str> = self.into();
        match option_text {
            Some(text) => text == other,
            None => false,
        }
    }
}
// To make comparison bidirectional
impl PartialEq<Value> for str {
    fn eq(&self, other: &Value) -> bool {
        other == self
    }
}

impl<'o> Into<Option<&'o str>> for &'o Value {
    fn into(self) -> Option<&'o str> {
        match self {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }
}

pub fn ref_to_option_str(value: &Value) -> Option<&str> {
    value.into()
}

pub fn to_option_str<'o>(value: &'o Option<Cow<'o, Value>>) -> Option<&'o str> {
    match value {
        Some(cow) => cow.as_ref().into(),
        None => None,
    }
}

#[cfg(test)]
extern crate serde_json;

#[cfg(test)]
mod test {
    use super::*;
    use chrono::prelude::*;
    use serde_json;
    use std::fs;

    #[test]
    fn created_ts_should_be_iso_8601() {
        // Arrange
        let event = Event::new("");
        let created_ts = event.created_ts;
        println!("created_ts: [{}]", created_ts);

        // Act
        let dt = DateTime::parse_from_rfc3339(&created_ts);

        // Assert
        assert!(dt.is_ok());
    }

    #[test]
    fn should_return_an_option_with_text() {
        // Arrange
        let value = Value::Text("text_value".to_owned());

        // Act
        let text: Option<&str> = (&value).into();

        // Assert
        assert!(text.is_some());
        assert_eq!("text_value", text.unwrap());
    }

    #[test]
    fn should_return_an_empty_option() {
        // Arrange
        let value = Value::Map(HashMap::new());

        // Act
        let text: Option<&str> = (&value).into();

        // Assert
        assert!(text.is_none());
    }

    #[test]
    fn should_return_an_option_with_text_from_cow() {
        // Arrange
        let value = Value::Text("text_value".to_owned());
        let cow = Cow::Borrowed(&value);

        // Act
        let text = ref_to_option_str(&cow);

        // Assert
        assert!(text.is_some());
        assert_eq!("text_value", text.unwrap());
    }

    #[test]
    fn should_return_an_option_with_text_from_option() {
        // Arrange
        let value = Value::Text("text_value".to_owned());
        let option = Some(Cow::Borrowed(&value));

        // Act
        let text = to_option_str(&option);

        // Assert
        assert!(text.is_some());
        assert_eq!("text_value", text.unwrap());
    }

    #[test]
    fn should_compare_value_with_str() {
        // Arrange
        let value = Value::Text("text_value".to_owned());

        // Assert
        assert_eq!("text_value", &value);
        assert_eq!(&value, "text_value");
    }

    #[test]
    fn should_parse_a_json_event_with_nested_payload() {
        // Arrange
        let filename = "./test_resources/event_nested_01.json";
        let event_json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

        // Act
        let event = serde_json::from_str::<Event>(&event_json);

        // Assert
        assert!(event.is_ok());
    }

    #[test]
    fn value_text_should_return_no_child() {
        // Arrange
        let value = Value::Text("".to_owned());

        // Act
        let result = value.get_from_map("");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn value_map_should_return_child_if_exists() {
        // Arrange
        let mut children = HashMap::new();
        children.insert("first".to_owned(), Value::Text("first_value".to_owned()));
        children.insert("second".to_owned(), Value::Text("second_value".to_owned()));

        let value = Value::Map(children);

        // Act
        let result = value.get_from_map("second");

        // Assert
        assert!(result.is_some());
        assert_eq!("second_value", result.unwrap());
    }

    #[test]
    fn value_map_should_return_no_child_if_absent() {
        // Arrange
        let mut children = HashMap::new();
        children.insert("first".to_owned(), Value::Text("first_value".to_owned()));
        children.insert("second".to_owned(), Value::Text("second_value".to_owned()));

        let value = Value::Map(children);

        // Act
        let result = value.get_from_map("third");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn should_convert_event_into_type() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("one-key".to_owned(), Value::Text("one-value".to_owned()));
        payload.insert("two-key".to_owned(), Value::Text("two-value".to_owned()));

        let event = Event::new_with_payload("my-event-type", payload.clone());
        let created_ts = event.created_ts.to_owned();

        // Act
        let event_value: Value = event.into();

        // Assert
        assert_eq!("my-event-type", event_value.get_from_map("type").unwrap().text().unwrap());
        assert_eq!(created_ts, event_value.get_from_map("created_ts").unwrap().text().unwrap());
        assert_eq!(&Value::Map(payload), event_value.get_from_map("payload").unwrap());
    }
}
