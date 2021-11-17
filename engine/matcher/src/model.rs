use crate::error::MatcherError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tornado_common_api::{Action, Event, Number, Payload, Value, ValueExt};
use typescript_definitions::TypeScriptify;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct InternalEvent {
    pub trace_id: String,
    #[serde(rename = "type")]
    pub event_type: Value,
    pub created_ms: Value,
    pub metadata: Value,
    pub payload: Value,
}

impl From<Event> for InternalEvent {
    fn from(event: Event) -> Self {
        InternalEvent {
            trace_id: event.trace_id,
            event_type: Value::Text(event.event_type),
            created_ms: Value::Number(Number::PosInt(event.created_ms)),
            metadata: Value::Null,
            payload: Value::Map(event.payload),
        }
    }
}

impl From<InternalEvent> for Value {
    fn from(event: InternalEvent) -> Self {
        let mut payload = Payload::new();
        payload.insert("trace_id".to_owned(), Value::Text(event.trace_id));
        payload.insert("type".to_owned(), event.event_type);
        payload.insert("created_ms".to_owned(), event.created_ms);
        payload.insert("payload".to_owned(), event.payload);
        payload.insert("metadata".to_owned(), event.metadata);
        Value::Map(payload)
    }
}

/*
fn add_to_metadata(event: &mut Value, (key, value): (String, Value)) -> Result<(), MatcherError> {
    match &event.get_from_map("metadata") {
        None => {

            if let Some(map) = event.get_map_mut() {
                let mut payload = HashMap::new();
                payload.insert(key, value);
                map.insert("metadata".to_owned(), Value::Map(payload));
                Ok(())
            } else {
                Err(MatcherError::InternalSystemError {
                    message: "Event should be a Map".to_owned(),
                })
            }
        }
        Some(Value::Map(mut payload)) => {
            payload.insert(key, value);
            Ok(())
        }
        _ => Err(MatcherError::InternalSystemError {
            message: "InternalEvent metadata should be a Map".to_owned(),
        }),
    }
}
*/

impl InternalEvent {
    pub fn new(event: Event) -> Self {
        event.into()
    }

    pub fn add_to_metadata(&mut self, key: String, value: Value) -> Result<(), MatcherError> {
        match &mut self.metadata {
            Value::Null => {
                let mut payload = HashMap::new();
                payload.insert(key, value);
                self.metadata = Value::Map(payload);
                Ok(())
            }
            Value::Map(payload) => {
                payload.insert(key, value);
                Ok(())
            }
            _ => Err(MatcherError::InternalSystemError {
                message: "InternalEvent metadata should be a Map".to_owned(),
            }),
        }
    }
}

/// A ProcessedEvent is the result of the matcher process.
/// It contains the original Event along with the result of the matching operation.
#[derive(Debug, Clone)]
pub struct ProcessedEvent {
    pub event: InternalEvent,
    pub result: ProcessedNode,
}

#[derive(Debug, Clone)]
pub enum ProcessedNode {
    Filter { name: String, filter: ProcessedFilter, nodes: Vec<ProcessedNode> },
    Ruleset { name: String, rules: ProcessedRules },
}

#[derive(Debug, Clone)]
pub struct ProcessedFilter {
    pub status: ProcessedFilterStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessedFilterStatus {
    Matched,
    NotMatched,
    Inactive,
}
#[derive(Debug, Clone)]
pub struct ProcessedRules {
    pub rules: Vec<ProcessedRule>,
    pub extracted_vars: Value,
}

#[derive(Debug, Clone)]
pub struct ProcessedRule {
    pub name: String,
    pub status: ProcessedRuleStatus,
    pub actions: Vec<Action>,
    pub message: Option<String>,
    pub meta: Option<ProcessedRuleMetaData>,
}

impl ProcessedRule {
    pub fn new(rule_name: String) -> ProcessedRule {
        ProcessedRule {
            name: rule_name,
            status: ProcessedRuleStatus::NotProcessed,
            actions: vec![],
            message: None,
            meta: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessedRuleStatus {
    Matched,
    PartiallyMatched,
    NotMatched,
    NotProcessed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypeScriptify)]
pub struct ProcessedRuleMetaData {
    pub actions: Vec<ActionMetaData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypeScriptify)]
pub struct ActionMetaData {
    pub id: String,
    pub payload: HashMap<String, EnrichedValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TypeScriptify)]
pub struct EnrichedValue {
    pub content: EnrichedValueContent,
    pub meta: ValueMetaData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TypeScriptify)]
#[serde(tag = "type")]
pub enum EnrichedValueContent {
    Single { content: Value },
    Map { content: HashMap<String, EnrichedValue> },
    Array { content: Vec<EnrichedValue> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TypeScriptify)]
pub struct ValueMetaData {
    pub modified: bool,
    pub is_leaf: bool,
}

#[cfg(test)]
mod test {
    use crate::model::InternalEvent;
    use tornado_common_api::{Event, Number, Payload, Value};

    #[test]
    fn should_convert_between_event_and_internal_event() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("one-key".to_owned(), Value::Text("one-value".to_owned()));
        payload.insert("number".to_owned(), Value::Number(Number::from_f64(999.99).unwrap()));
        payload.insert("bool".to_owned(), Value::Bool(false));

        let event = Event::new_with_payload("my-event-type", payload.clone());

        // Act
        let internal_from_event: InternalEvent = event.clone().into();
        let json_from_internal = serde_json::to_string(&internal_from_event).unwrap();
        let event_from_internal: Event = serde_json::from_str(&json_from_internal).unwrap();

        // Assert
        assert_eq!(event, event_from_internal);
    }

    #[test]
    fn should_create_and_add_metadata() {
        // Arrange

        let mut event = InternalEvent::new(Event::default());

        let key_1 = "random_key_1";
        let value_1 = Value::Number(Number::PosInt(123));

        let key_2 = "random_key_2";
        let value_2 = Value::Number(Number::Float(3.4));

        // Act
        event.add_to_metadata(key_1.to_owned(), value_1.clone()).unwrap();
        event.add_to_metadata(key_2.to_owned(), value_2.clone()).unwrap();

        // Assert
        match event.metadata {
            Value::Map(payload) => {
                assert_eq!(2, payload.len());
                assert_eq!(&value_1, payload.get(key_1).unwrap());
                assert_eq!(&value_2, payload.get(key_2).unwrap());
            }
            _ => assert!(false),
        }
    }

    /*
    #[test]
    fn should_create_and_add_metadata() {
        // Arrange

        let mut event = serde_json::to_value(Event::default()).unwrap();

        let key_1 = "random_key_1";
        let value_1 = Value::Number(Number::PosInt(123));

        let key_2 = "random_key_2";
        let value_2 = Value::Number(Number::Float(3.4));

        // Act
        add_to_metadata(&mut event, (key_1.to_owned(), value_1.clone())).unwrap();
        add_to_metadata(&mut event, (key_2.to_owned(), value_2.clone())).unwrap();

        // Assert
        match event.metadata {
            Value::Map(payload) => {
                assert_eq!(2, payload.len());
                assert_eq!(&value_1, payload.get(key_1).unwrap());
                assert_eq!(&value_2, payload.get(key_2).unwrap());
            }
            _ => assert!(false),
        }
    }
    */

    #[test]
    fn should_create_and_override_metadata() {
        // Arrange

        let mut event = InternalEvent::new(Event::default());

        let key_1 = "random_key_1";
        let value_1 = Value::Number(Number::PosInt(123));

        let value_2 = Value::Number(Number::Float(3.4));

        // Act
        event.add_to_metadata(key_1.to_owned(), value_1.clone()).unwrap();
        event.add_to_metadata(key_1.to_owned(), value_2.clone()).unwrap();

        // Assert
        match event.metadata {
            Value::Map(payload) => {
                assert_eq!(1, payload.len());
                assert_eq!(&value_2, payload.get(key_1).unwrap());
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_fail_if_metadata_is_not_map() {
        // Arrange

        let mut event = InternalEvent::new(Event::default());
        event.metadata = Value::Array(vec![]);

        let key_1 = "random_key_1";
        let value_1 = Value::Number(Number::PosInt(123));

        // Act
        let result = event.add_to_metadata(key_1.to_owned(), value_1.clone());

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_convert_to_value_and_back() {
        // Arrange

        let mut event = InternalEvent::new(Event::default());
        event.metadata = Value::Array(vec![]);

        // Act
        let value: Value = event.clone().into();
        let event_from_value: InternalEvent =
            serde_json::from_value(serde_json::to_value(value).unwrap()).unwrap();

        // Assert
        assert_eq!(event, event_from_value);
    }
}
