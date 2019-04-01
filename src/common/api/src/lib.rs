use chrono::prelude::Local;
use serde_derive::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;

/// An Event is correlated with an incoming episode, incident, situation or any kind of message
///   that could be meaningful to the system.
/// Events are produced by Collectors and are sent to the Tornado Engine to be processed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub created_ms: u64,
    pub payload: Payload,
}

impl Event {
    pub fn new<S: Into<String>>(event_type: S) -> Event {
        Event::new_with_payload(event_type, HashMap::new())
    }

    pub fn new_with_payload<S: Into<String>>(event_type: S, payload: Payload) -> Event {
        let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
        let created_ms = dt.timestamp_millis() as u64;
        Event { event_type: event_type.into(), created_ms, payload }
    }
}

impl Into<Value> for Event {
    fn into(self) -> Value {
        let mut payload = Payload::new();
        payload.insert("type".to_owned(), Value::Text(self.event_type));
        payload.insert("created_ms".to_owned(), Value::Number(Number::PosInt(self.created_ms)));
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
    Null,
    Bool(bool),
    Number(Number),
    Map(Payload),
    Array(Vec<Value>),
}

#[derive(Copy, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Number {
    PosInt(u64),
    /// Always less than zero.
    NegInt(i64),
    /// Always finite.
    Float(f64),
}

impl Number {

    #[inline]
    pub fn is_i64(&self) -> bool {
            match self {
                Number::PosInt(v) => v <= &(i64::max_value() as u64),
                Number::NegInt(_) => true,
                Number::Float(_) => false,
        }
    }

    #[inline]
    pub fn is_u64(&self) -> bool {
            match self {
                Number::PosInt(_) => true,
                Number::NegInt(_) | Number::Float(_) => false,
        }
    }

    #[inline]
    pub fn is_f64(&self) -> bool {
            match self {
                Number::Float(_) => true,
                Number::PosInt(_) | Number::NegInt(_) => false,
        }
    }

    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
            match self {
                Number::PosInt(n) => {
                    let n = *n;
                if n <= i64::max_value() as u64 {
                    Some(n as i64)
                } else {
                    None
                }
            }
                Number::NegInt(n) => Some(*n),
                Number::Float(_) => None,
        }
    }

    #[inline]
    pub fn as_u64(&self) -> Option<u64> {
            match self {
                Number::PosInt(n) => Some(*n),
                Number::NegInt(_) | Number::Float(_) => None,
        }
    }

    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
            match self {
                Number::PosInt(n) => Some(*n as f64),
                Number::NegInt(n) => Some(*n as f64),
                Number::Float(n) => Some(*n),
        }
    }

    #[inline]
    pub fn from_f64(f: f64) -> Option<Number> {
        if f.is_finite() {
            Some( Number::Float(f))
        } else {
            None
        }
    }

}

impl Value {
    pub fn get_from_map(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(payload) => payload.get(key),
            _ => None,
        }
    }
    pub fn get_from_array(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Array(array) => array.get(index),
            _ => None,
        }
    }
    pub fn get_map(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Map(payload) => Some(payload),
            _ => None,
        }
    }
    pub fn get_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(array) => Some(array),
            _ => None,
        }
    }
    pub fn get_text(&self) -> Option<&str> {
        match self {
            Value::Text(value) => Some(value),
            _ => None,
        }
    }
    pub fn get_bool(&self) -> Option<&bool> {
        match self {
            Value::Bool(value) => Some(value),
            _ => None,
        }
    }
    pub fn get_number(&self) -> Option<&Number> {
        match self {
            Value::Number(value) => Some(value),
            _ => None,
        }
    }
}

// Allows str == Value
impl PartialEq<str> for Value {
    fn eq(&self, other: &str) -> bool {
        let option_text = self.get_text();
        match option_text {
            Some(text) => text == other,
            None => false,
        }
    }
}

// Allows Value == str
impl PartialEq<Value> for str {
    fn eq(&self, other: &Value) -> bool {
        other == self
    }
}

// Allows bool == Value
impl PartialEq<bool> for Value {
    fn eq(&self, other: &bool) -> bool {
        let option_bool = self.get_bool();
        match option_bool {
            Some(value) => value == other,
            None => false,
        }
    }
}

// Allows Value == bool
impl PartialEq<Value> for bool {
    fn eq(&self, other: &Value) -> bool {
        other == self
    }
}

// Allows f64 == Value
impl PartialEq<f64> for Value {
    fn eq(&self, other: &f64) -> bool {
        let option_number = self.get_number();
        match option_number {
            Some(value) => value.as_f64() == Some(*other),
            None => false,
        }
    }
}

// Allows Value == f64
impl PartialEq<Value> for f64 {
    fn eq(&self, other: &Value) -> bool {
        other == self
    }
}

// Allows u64 == Value
impl PartialEq<u64> for Value {
    fn eq(&self, other: &u64) -> bool {
        let option_number = self.get_number();
        match option_number {
            Some(value) => value.as_u64() == Some(*other),
            None => false,
        }
    }
}

// Allows Value == u64
impl PartialEq<Value> for u64 {
    fn eq(&self, other: &Value) -> bool {
        other == self
    }
}

// Allows i64 == Value
impl PartialEq<i64> for Value {
    fn eq(&self, other: &i64) -> bool {
        let option_number = self.get_number();
        match option_number {
            Some(value) => value.as_i64() == Some(*other),
            None => false,
        }
    }
}

// Allows Value == i64
impl PartialEq<Value> for i64 {
    fn eq(&self, other: &Value) -> bool {
        other == self
    }
}

pub fn cow_to_str<'o>(value: &'o Option<Cow<'o, Value>>) -> Option<&'o str> {
    match value {
        Some(cow) => cow.as_ref().get_text(),
        None => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn created_ms_should_be_preset() {
        // Arrange
        let before_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        // Act
        let event = Event::new("");
        let created_ms = event.created_ms as u128;
        println!("created_ms: [{}]", created_ms);

        let after_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

        // Assert
        assert!(created_ms >= before_ms);
        assert!(created_ms <= after_ms);
    }

    #[test]
    fn should_return_an_option_with_text() {
        // Arrange
        let value = Value::Text("text_value".to_owned());

        // Act
        let text = (&value).get_text();

        // Assert
        assert!(text.is_some());
        assert_eq!("text_value", text.unwrap());
    }

    #[test]
    fn should_return_an_empty_option() {
        // Arrange
        let value = Value::Map(HashMap::new());

        // Act
        let text = (&value).get_text();

        // Assert
        assert!(text.is_none());
    }

    #[test]
    fn should_return_an_option_with_bool() {
        // Arrange
        let value = Value::Bool(true);

        // Act
        let boolean = value.get_bool();

        // Assert
        assert!(boolean.is_some());
        assert!(boolean.unwrap());
    }

    #[test]
    fn should_return_an_option_with_number() {
        // Arrange
        let value = Value::Number(Number::Float(64.0));

        // Act
        let number = value.get_number();

        // Assert
        assert!(number.is_some());
        assert_eq!(64.0, number.unwrap().as_f64().unwrap());
    }

    #[test]
    fn should_return_an_option_with_text_from_cow() {
        // Arrange
        let value = Value::Text("text_value".to_owned());
        let cow = Cow::Borrowed(&value);

        // Act
        let text = cow.get_text();

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
        let text = cow_to_str(&option);

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
    fn should_compare_value_with_bool() {
        // Arrange
        let value = Value::Bool(true);

        // Assert
        assert_eq!(&true, &value);
        assert_eq!(&value, &true);
    }

    #[test]
    fn should_compare_value_with_f64() {
        // Arrange
        let value = Value::Number(Number::Float(69.0));

        // Assert
        assert_eq!(&69.0, &value);
        assert_eq!(&value, &69.0);
    }

    #[test]
    fn should_compare_value_with_u64() {
        // Arrange
        let u_value: u64 = 69;
        let value = Value::Number(Number::PosInt(u_value));

        // Assert
        assert_eq!(&u_value, &value);
        assert_eq!(&value, &u_value);
    }

    #[test]
    fn should_compare_value_with_i64() {
        // Arrange
        let i_value: i64 = -69;
        let value = Value::Number(Number::NegInt(i_value));

        // Assert
        assert_eq!(&i_value, &value);
        assert_eq!(&value, &i_value);
    }

    #[test]
    fn should_compare_array_values() {
        // Arrange
        let value_1 = Value::Array(vec![Value::Text("text_value".to_owned()), Value::Bool(false)]);

        // Assert
        assert_ne!(Value::Array(vec![]), value_1);
        assert_eq!(value_1.clone(), value_1);
    }

    #[test]
    fn should_compare_map_values() {
        // Arrange
        let array = Value::Array(vec![Value::Text("text_value".to_owned()), Value::Bool(false)]);

        let mut payload = Payload::new();
        payload.insert("array".to_owned(), array);
        payload.insert("bool".to_owned(), Value::Bool(false));

        let map = Value::Map(payload.clone());

        // Assert
        assert_ne!(Value::Map(Payload::new()), map);
        assert_eq!(Value::Map(payload.clone()), map);
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
    fn should_parse_a_json_event_with_a_null_value() {
        // Arrange
        let filename = "./test_resources/event_with_null_value.json";
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
        payload.insert("number".to_owned(), Value::Number(Number::from_f64(999.99).unwrap()));
        payload.insert("bool".to_owned(), Value::Bool(false));

        let event = Event::new_with_payload("my-event-type", payload.clone());
        let created_ms = event.created_ms.to_owned();

        // Act
        let event_value: Value = event.into();

        // Assert
        assert_eq!("my-event-type", event_value.get_from_map("type").unwrap().get_text().unwrap());
        assert_eq!(&created_ms, event_value.get_from_map("created_ms").unwrap());
        assert_eq!(&Value::Map(payload), event_value.get_from_map("payload").unwrap());
    }
}
