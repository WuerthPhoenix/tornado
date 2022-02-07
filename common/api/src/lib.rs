use chrono::prelude::Local;
use error::CommonError;
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::sdk::propagation::TraceContextPropagator;
use opentelemetry::{Context, ContextGuard};
use partial_ordering::PartialOrdering;
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use tornado_common_logger::opentelemetry_logger::{
    TelemetryContextExtractor, TelemetryContextInjector,
};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub mod error;
pub mod partial_ordering;

pub type Value = serde_json::Value;
pub type Map<K, V> = serde_json::Map<K, V>;
pub type Number = serde_json::Number;

/// An Event is correlated with an incoming episode, incident, situation or any kind of message
///   that could be meaningful to the system.
/// Events are produced by Collectors and are sent to the Tornado Engine to be processed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Event {
    #[serde(default = "default_trace_id")]
    #[serde(deserialize_with = "deserialize_null_trace_id")]
    pub trace_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub created_ms: u64,
    pub payload: Payload,
    pub metadata: Option<Map<String, Value>>,
}

pub trait WithEventData {
    fn trace_id(&self) -> Option<&str>;
    fn event_type(&self) -> Option<&str>;
    fn created_ms(&self) -> Option<u64>;
    fn payload(&self) -> Option<&Payload>;
    fn metadata(&self) -> Option<&Value>;
    fn add_to_metadata(&mut self, key: String, value: Value) -> Result<(), CommonError>;
}

pub const EVENT_TRACE_ID: &str = "trace_id";
pub const EVENT_TYPE: &str = "type";
pub const EVENT_CREATED_MS: &str = "created_ms";
pub const EVENT_PAYLOAD: &str = "payload";
pub const EVENT_METADATA: &str = "metadata";
const METADATA_TRACE_CONTEXT: &str = "trace_context";

impl WithEventData for Value {
    fn trace_id(&self) -> Option<&str> {
        self.get(EVENT_TRACE_ID).and_then(|val| val.get_text())
    }

    fn event_type(&self) -> Option<&str> {
        self.get(EVENT_TYPE).and_then(|val| val.get_text())
    }

    fn created_ms(&self) -> Option<u64> {
        self.get(EVENT_CREATED_MS).and_then(|val| val.get_number()).and_then(|num| num.as_u64())
    }

    fn payload(&self) -> Option<&Payload> {
        self.get(EVENT_PAYLOAD).and_then(|val| val.get_map())
    }

    fn metadata(&self) -> Option<&Value> {
        self.get(EVENT_METADATA)
    }

    fn add_to_metadata(&mut self, key: String, value: Value) -> Result<(), CommonError> {
        match self.get_mut(EVENT_METADATA) {
            Some(Value::Null) | None => {
                if let Some(map) = self.get_map_mut() {
                    let mut payload = Map::new();
                    payload.insert(key, value);
                    map.insert(EVENT_METADATA.to_owned(), Value::Object(payload));
                    Ok(())
                } else {
                    Err(CommonError::BadDataError { message: "Event should be a Map".to_owned() })
                }
            }
            Some(Value::Object(payload)) => {
                payload.insert(key, value);
                Ok(())
            }
            _ => Err(CommonError::BadDataError {
                message: "Event metadata should be a Map".to_owned(),
            }),
        }
    }
}

#[inline]
fn default_trace_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn deserialize_null_trace_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_else(default_trace_id))
}

impl Event {
    pub fn new<S: Into<String>>(event_type: S) -> Event {
        Event::new_with_payload(event_type, Map::new())
    }

    pub fn new_with_payload<S: Into<String>>(event_type: S, payload: Payload) -> Event {
        let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
        let created_ms = dt.timestamp_millis() as u64;
        Event {
            trace_id: default_trace_id(),
            event_type: event_type.into(),
            created_ms,
            payload,
            metadata: None,
        }
    }

    pub fn get_trace_context(&self) -> Option<Context> {
        self.metadata
            .as_ref()
            .and_then(|val| val.get(METADATA_TRACE_CONTEXT))
            .and_then(|val| val.as_object())
            .map(TelemetryContextExtractor)
            .map(|context| TraceContextPropagator::new().extract(&context))
    }

    /// Sets the field metadata.trace_context of this event
    /// to the context of the passed span
    pub fn set_trace_context_from_span(&mut self, span: &Span) {
        let mut map = Map::new();
        let mut context_carrier = TelemetryContextInjector(&mut map);
        TraceContextPropagator::new().inject_context(&span.context(), &mut context_carrier);

        if !context_carrier.0.is_empty() {
            match self.metadata.as_mut() {
                None => {
                    let mut metadata = Map::new();
                    metadata.insert(METADATA_TRACE_CONTEXT.to_owned(), Value::Object(map));
                    self.metadata = Some(metadata)
                }
                Some(metadata) => {
                    metadata.insert(METADATA_TRACE_CONTEXT.to_owned(), Value::Object(map));
                }
            }
        }
    }

    /// Attaches the trace context contained in the event if present, and returns its guard
    pub fn attach_trace_context(&self) -> Option<ContextGuard> {
        self.get_trace_context().map(|context| context.attach())
    }
}

/// An Action is produced when an Event matches a specific Rule.
/// Once created, the Tornado Engine sends the Action to the Executors to be resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub trace_id: Option<String>,
    pub id: String,
    pub payload: Payload,
}

impl Action {
    pub fn new<T: Into<String>, S: Into<String>>(trace_id: T, id: S) -> Action {
        Action::new_with_payload(trace_id, id, Map::new())
    }
    pub fn new_with_payload<T: Into<String>, S: Into<String>>(
        trace_id: T,
        id: S,
        payload: Payload,
    ) -> Action {
        Action { trace_id: Some(trace_id.into()), id: id.into(), payload }
    }
}

pub type Payload = Map<String, Value>;

pub trait ValueGet {
    fn get_from_map(&self, key: &str) -> Option<&Value>;
    fn get_from_array(&self, index: usize) -> Option<&Value>;
}

pub trait ValueExt {
    fn get_map(&self) -> Option<&Map<String, Value>>;

    fn get_map_mut(&mut self) -> Option<&mut Map<String, Value>>;

    fn get_array(&self) -> Option<&Vec<Value>>;

    fn get_text(&self) -> Option<&str>;

    fn get_bool(&self) -> Option<&bool>;

    fn get_number(&self) -> Option<&Number>;
}

impl ValueGet for Value {
    fn get_from_map(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(payload) => payload.get(key),
            _ => None,
        }
    }
    fn get_from_array(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Array(array) => array.get(index),
            _ => None,
        }
    }
}

impl ValueExt for Value {
    fn get_map(&self) -> Option<&Map<String, Value>> {
        match self {
            Value::Object(payload) => Some(payload),
            _ => None,
        }
    }

    fn get_map_mut(&mut self) -> Option<&mut Map<String, Value>> {
        match self {
            Value::Object(payload) => Some(payload),
            _ => None,
        }
    }

    fn get_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(array) => Some(array),
            _ => None,
        }
    }

    fn get_text(&self) -> Option<&str> {
        match self {
            Value::String(value) => Some(value),
            _ => None,
        }
    }

    fn get_bool(&self) -> Option<&bool> {
        match self {
            Value::Bool(value) => Some(value),
            _ => None,
        }
    }

    fn get_number(&self) -> Option<&Number> {
        match self {
            Value::Number(value) => Some(value),
            _ => None,
        }
    }
}

pub fn cow_to_str<'o>(value: &'o Option<Cow<'o, Value>>) -> Option<&'o str> {
    match value {
        Some(cow) => cow.as_ref().get_text(),
        None => None,
    }
}

pub fn partial_cmp_option_cow_value<'o, F: FnOnce() -> Option<Cow<'o, Value>>>(
    first: &'o Option<Cow<'o, Value>>,
    second: F,
) -> Option<Ordering> {
    if let Some(first_value) = first {
        if let Some(second_value) = second() {
            first_value.as_ref().partial_cmp(second_value.as_ref())
        } else {
            None
        }
    } else {
        None
    }
}

pub trait RetriableError {
    fn can_retry(&self) -> bool;
}

impl<'o> ValueGet for HashMap<&'o str, &'o Value> {
    fn get_from_map(&self, key: &str) -> Option<&Value> {
        self.get(key).map(|s| *s)
    }
    fn get_from_array(&self, _index: usize) -> Option<&Value> {
        None
    }
}

impl ValueGet for Map<String, Value> {
    fn get_from_map(&self, key: &str) -> Option<&Value> {
        self.get(key)
    }
    fn get_from_array(&self, _index: usize) -> Option<&Value> {
        None
    }
}

impl ValueGet for HashMap<String, Value> {
    fn get_from_map(&self, key: &str) -> Option<&Value> {
        self.get(key)
    }
    fn get_from_array(&self, _index: usize) -> Option<&Value> {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use opentelemetry::trace::TraceContextExt;
    use serde_json::{self, json};
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
        let value = Value::String("text_value".to_owned());

        // Act
        let text = (&value).get_text();

        // Assert
        assert!(text.is_some());
        assert_eq!("text_value", text.unwrap());
    }

    #[test]
    fn should_return_an_empty_option() {
        // Arrange
        let value = Value::Object(Map::new());

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
        let value = json!(64.0);

        // Act
        let number = value.get_number();

        // Assert
        assert!(number.is_some());
        assert_eq!(Some(64.0), number.unwrap().as_f64());
    }

    #[test]
    fn should_return_an_option_with_text_from_cow() {
        // Arrange
        let value = Value::String("text_value".to_owned());
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
        let value = Value::String("text_value".to_owned());
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
        let value = Value::String("text_value".to_owned());

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
        let value = json!(69.0);

        // Assert
        assert_eq!(&69.0, &value);
        assert_eq!(&value, &69.0);
    }

    #[test]
    fn should_compare_value_with_u64() {
        // Arrange
        let u_value: u64 = 69;
        let value = json!(u_value);

        // Assert
        assert_eq!(&u_value, &value);
        assert_eq!(&value, &u_value);
    }

    #[test]
    fn should_compare_value_with_i64() {
        // Arrange
        let i_value: i64 = -69;
        let value = json!(i_value);

        // Assert
        assert_eq!(&i_value, &value);
        assert_eq!(&value, &i_value);
    }

    #[test]
    fn should_compare_array_values() {
        // Arrange
        let value_1 =
            Value::Array(vec![Value::String("text_value".to_owned()), Value::Bool(false)]);

        // Assert
        assert_ne!(Value::Array(vec![]), value_1);
        assert_eq!(value_1.clone(), value_1);
    }

    #[test]
    fn should_compare_map_values() {
        // Arrange
        let array = Value::Array(vec![Value::String("text_value".to_owned()), Value::Bool(false)]);

        let mut payload = Payload::new();
        payload.insert("array".to_owned(), array);
        payload.insert("bool".to_owned(), Value::Bool(false));

        let map = Value::Object(payload.clone());

        // Assert
        assert_ne!(Value::Object(Payload::new()), map);
        assert_eq!(Value::Object(payload.clone()), map);
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
    fn should_parse_numbers_as_float() {
        // Arrange
        let filename = "./test_resources/event_nested_01.json";
        let event_json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

        // Act
        let event = serde_json::from_str::<Event>(&event_json).unwrap();

        // Assert
        assert_eq!(&json!(123456.789), event.payload.get("number_f64").unwrap());
    }

    #[test]
    fn should_parse_numbers_as_neg_in() {
        // Arrange
        let filename = "./test_resources/event_nested_01.json";
        let event_json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

        // Act
        let event = serde_json::from_str::<Event>(&event_json).unwrap();

        // Assert
        assert_eq!(&json!(-111), event.payload.get("number_i64").unwrap());
    }

    #[test]
    fn should_parse_numbers_as_pos_int() {
        // Arrange
        let filename = "./test_resources/event_nested_01.json";
        let event_json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

        // Act
        let event = serde_json::from_str::<Event>(&event_json).unwrap();

        // Assert
        assert_eq!(&json!(222), event.payload.get("number_u64").unwrap());
    }

    #[test]
    fn value_text_should_return_no_child() {
        // Arrange
        let value = Value::String("".to_owned());

        // Act
        let result = value.get_from_map("");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn value_map_should_return_child_if_exists() {
        // Arrange
        let mut children = Map::new();
        children.insert("first".to_owned(), Value::String("first_value".to_owned()));
        children.insert("second".to_owned(), Value::String("second_value".to_owned()));

        let value = Value::Object(children);

        // Act
        let result = value.get_from_map("second");

        // Assert
        assert!(result.is_some());
        assert_eq!("second_value", result.unwrap());
    }

    #[test]
    fn value_map_should_return_no_child_if_absent() {
        // Arrange
        let mut children = Map::new();
        children.insert("first".to_owned(), Value::String("first_value".to_owned()));
        children.insert("second".to_owned(), Value::String("second_value".to_owned()));

        let value = Value::Object(children);

        // Act
        let result = value.get_from_map("third");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn should_convert_event_into_value() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("one-key".to_owned(), Value::String("one-value".to_owned()));
        payload.insert("two-key".to_owned(), Value::String("two-value".to_owned()));
        payload.insert("number".to_owned(), Value::Number(Number::from_f64(999.99).unwrap()));
        payload.insert("bool".to_owned(), Value::Bool(false));

        let event = Event::new_with_payload("my-event-type", payload.clone());
        let created_ms = event.created_ms.to_owned();

        // Act
        let value_from_event: Value = json!(event);

        // Assert
        assert_eq!(
            "my-event-type",
            value_from_event.get_from_map("type").unwrap().get_text().unwrap()
        );
        assert_eq!(&created_ms, value_from_event.get_from_map("created_ms").unwrap());
        assert_eq!(&Value::Object(payload), value_from_event.get_from_map("payload").unwrap());
    }

    #[test]
    fn should_convert_between_event_and_value() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("one-key".to_owned(), Value::String("one-value".to_owned()));
        payload.insert("number".to_owned(), Value::Number(Number::from_f64(999.99).unwrap()));
        payload.insert("bool".to_owned(), Value::Bool(false));

        let event = Event::new_with_payload("my-event-type", payload.clone());

        // Act
        let value_from_event: Value = json!(event.clone());
        let json_from_value = serde_json::to_string(&value_from_event).unwrap();
        let event_from_value: Event = serde_json::from_str(&json_from_value).unwrap();

        // Assert
        assert_eq!(event, event_from_value);
    }

    #[test]
    fn should_partially_cmp_values() {
        // Text
        assert_eq!(
            Some(Ordering::Equal),
            Value::String("one".to_owned()).partial_cmp(&Value::String("one".to_owned()))
        );
        assert_eq!(
            Some(Ordering::Greater),
            Value::String("two".to_owned()).partial_cmp(&Value::String("one".to_owned()))
        );
        assert_eq!(
            Some(Ordering::Less),
            Value::String("one".to_owned()).partial_cmp(&Value::String("two".to_owned()))
        );
        assert_eq!(None, Value::String("one".to_owned()).partial_cmp(&Value::Bool(true)));

        // Bool
        assert_eq!(Some(Ordering::Equal), Value::Bool(true).partial_cmp(&Value::Bool(true)));
        assert_eq!(Some(Ordering::Equal), Value::Bool(false).partial_cmp(&Value::Bool(false)));
        assert_eq!(Some(Ordering::Greater), Value::Bool(true).partial_cmp(&Value::Bool(false)));
        assert_eq!(Some(Ordering::Less), Value::Bool(false).partial_cmp(&Value::Bool(true)));
        assert_eq!(None, Value::Bool(true).partial_cmp(&Value::Array(vec![])));

        // Num
        assert_eq!(Some(Ordering::Equal), json!(64).partial_cmp(&json!(64)));
        assert_eq!(Some(Ordering::Equal), json!(64).partial_cmp(&json!(64)));
        assert_eq!(Some(Ordering::Equal), json!(64).partial_cmp(&json!(64)));
        assert_eq!(Some(Ordering::Equal), json!(64).partial_cmp(&json!(64)));
        assert_eq!(Some(Ordering::Equal), json!(64.0).partial_cmp(&json!(64.0)));
        assert_eq!(Some(Ordering::Equal), json!(0.0).partial_cmp(&json!(0)));
        assert_eq!(Some(Ordering::Equal), json!(0.0).partial_cmp(&json!(0)));

        assert_eq!(Some(Ordering::Greater), json!(0.0).partial_cmp(&json!(-1000)));
        assert_eq!(Some(Ordering::Greater), json!(0).partial_cmp(&json!(-1000)));
        assert_eq!(Some(Ordering::Greater), json!(0.0).partial_cmp(&json!(-100000.0)));

        assert_eq!(Some(Ordering::Less), json!(10).partial_cmp(&json!(1000)));
        assert_eq!(Some(Ordering::Less), json!(0.0).partial_cmp(&json!(1000)));

        assert_eq!(None, json!(0).partial_cmp(&Value::Bool(false)));

        // Array
        assert_eq!(Some(Ordering::Equal), Value::Array(vec![]).partial_cmp(&Value::Array(vec![])));
        assert_eq!(
            Some(Ordering::Greater),
            Value::Array(vec![Value::Bool(true)])
                .partial_cmp(&Value::Array(vec![Value::Bool(false)]))
        );
        assert_eq!(
            Some(Ordering::Less),
            Value::Array(vec![Value::Bool(true), Value::Bool(false)])
                .partial_cmp(&Value::Array(vec![Value::Bool(true), Value::Bool(true)]))
        );
    }

    #[test]
    fn should_return_a_valid_uuid_as_trace_id() {
        assert!(uuid::Uuid::parse_str(&default_trace_id()).is_ok());
    }

    #[test]
    fn deserializing_an_event_should_generate_trace_if_missing() {
        // Arrange
        let json = r#"
{
  "type": "email",
  "created_ms": 1554130814854,
  "payload":{}
}
        "#;

        // Act
        let event: Event = serde_json::from_str(json).expect("should add a trace_id");

        // Assert
        assert!(!event.trace_id.is_empty());
        assert!(uuid::Uuid::parse_str(&event.trace_id).is_ok());
    }

    #[test]
    fn deserializing_an_event_should_generate_trace_if_null() {
        // Arrange
        let json = r#"
{
  "trace_id": null,
  "type": "email",
  "created_ms": 1554130814854,
  "payload":{}
}
        "#;

        // Act
        let event: Event = serde_json::from_str(json).expect("should add a trace_id");

        // Assert
        assert!(!event.trace_id.is_empty());
        assert!(uuid::Uuid::parse_str(&event.trace_id).is_ok());
    }

    #[test]
    fn deserializing_an_event_should_use_trace_if_present() {
        // Arrange
        let json = r#"
{
  "trace_id": "abcdefghilmon",
  "type": "email",
  "created_ms": 1554130814854,
  "payload":{}
}
        "#;

        // Act
        let event: Event = serde_json::from_str(json).expect("should add a trace_id");

        // Assert
        assert_eq!("abcdefghilmon", event.trace_id);
    }

    #[test]
    fn generating_an_event_should_include_trace_id() {
        // Act
        let event = Event::new("hello");

        // Assert
        assert!(!event.trace_id.is_empty());
        assert!(uuid::Uuid::parse_str(&event.trace_id).is_ok());
    }

    #[test]
    fn should_get_event_data_from_value() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("one-key".to_owned(), Value::String("one-value".to_owned()));
        payload.insert("two-key".to_owned(), Value::String("two-value".to_owned()));
        payload.insert("number".to_owned(), Value::Number(Number::from_f64(999.99).unwrap()));
        payload.insert("bool".to_owned(), Value::Bool(false));

        let event = Event::new_with_payload("my-event-type", payload.clone());
        let created_ms = event.created_ms.to_owned();

        // Act
        let value: Value = json!(event.clone());

        // Assert
        assert_eq!(Some(created_ms), value.created_ms());
        assert_eq!(Some(event.event_type.as_str()), value.event_type());
        assert_eq!(Some(event.trace_id.as_str()), value.trace_id());
        assert_eq!(Some(&payload), value.payload());
        assert_eq!(value.metadata().unwrap(), &Value::Null);
    }

    #[test]
    fn should_create_event_and_add_metadata() {
        // Arrange

        let mut event = json!(Event::default());

        let key_1 = "random_key_1";
        let value_1 = json!(123);

        let key_2 = "random_key_2";
        let value_2 = json!(3.4);

        // Act
        event.add_to_metadata(key_1.to_owned(), value_1.clone()).unwrap();
        event.add_to_metadata(key_2.to_owned(), value_2.clone()).unwrap();

        // Assert
        match event.metadata() {
            Some(Value::Object(payload)) => {
                assert_eq!(2, payload.len());
                assert_eq!(&value_1, payload.get(key_1).unwrap());
                assert_eq!(&value_2, payload.get(key_2).unwrap());
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_create_event_and_override_metadata() {
        // Arrange

        let mut event = json!(Event::default());

        let key_1 = "random_key_1";
        let value_1 = json!(123);

        let value_2 = json!(3.4);

        // Act
        event.add_to_metadata(key_1.to_owned(), value_1.clone()).unwrap();
        event.add_to_metadata(key_1.to_owned(), value_2.clone()).unwrap();

        // Assert
        match event.metadata() {
            Some(Value::Object(payload)) => {
                assert_eq!(1, payload.len());
                assert_eq!(&value_2, payload.get(key_1).unwrap());
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn get_trace_context_should_return_none_if_metadata_is_none() {
        // Arrange
        let event = Event::default();

        // Act
        let res = event.get_trace_context();

        // Assert
        assert!(res.is_none())
    }

    #[test]
    fn get_trace_context_should_return_none_if_metadata_does_not_contain_trace_context() {
        // Arrange
        let mut event = Event::default();
        let mut metadata = Map::new();
        metadata.insert("some_metadata".to_owned(), Value::Number(Number::from(1)));
        event.metadata = Some(metadata);

        // Act
        let res = event.get_trace_context();

        // Assert
        assert!(res.is_none())
    }

    #[test]
    fn get_trace_context_should_return_empty_context_if_trace_context_is_not_in_correct_format() {
        // Arrange
        let mut event = Event::default();
        let mut trace_context = Map::new();
        trace_context.insert(
            "some_field1".to_owned(),
            Value::String(format!("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01")),
        );
        trace_context.insert("some_field2".to_owned(), Value::String("".to_owned()));

        let mut metadata = Map::new();

        metadata.insert("trace_context".to_owned(), Value::Object(trace_context));
        event.metadata = Some(metadata);

        // Act
        let res = event.get_trace_context();
        let trace_id = res.unwrap().span().span_context().trace_id().to_hex();

        // Assert
        assert_eq!(trace_id, "00000000000000000000000000000000".to_owned())
    }

    #[test]
    fn get_trace_context_should_return_trace_context() {
        // Arrange
        let mut event = Event::default();
        let mut trace_context = Map::new();
        let expected_trace_id = "0af7651916cd43dd8448eb211c80319c";
        trace_context.insert(
            "traceparent".to_owned(),
            Value::String(format!("00-{}-b7ad6b7169203331-01", expected_trace_id)),
        );
        trace_context.insert("tracestate".to_owned(), Value::String("".to_owned()));

        let mut metadata = Map::new();

        metadata.insert("trace_context".to_owned(), Value::Object(trace_context));
        event.metadata = Some(metadata);

        // Act
        let res = event.get_trace_context();
        let trace_id = res.unwrap().span().span_context().trace_id().to_hex();

        // Assert
        assert_eq!(trace_id, expected_trace_id.to_owned())
    }
}
