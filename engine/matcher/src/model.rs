use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tornado_common_api::{Action, Event, Number, Payload, Value};
use typescript_definitions::TypeScriptify;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct InternalEvent {
    pub trace_id: String,
    #[serde(rename = "type")]
    pub event_type: Value,
    pub created_ms: Value,
    pub payload: Value,
}

impl From<Event> for InternalEvent {
    fn from(event: Event) -> Self {
        InternalEvent {
            trace_id: event.trace_id,
            event_type: Value::Text(event.event_type),
            created_ms: Value::Number(Number::PosInt(event.created_ms)),
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
        Value::Map(payload)
    }
}

impl InternalEvent {
    pub fn new(event: Event) -> Self {
        event.into()
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
    use tornado_common_api::{Payload, Value, Number, Event};
    use crate::model::InternalEvent;

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
}