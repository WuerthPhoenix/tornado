use std::collections::HashMap;
use tornado_common_api::{Action, Event, Payload, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct InternalEvent {
    pub event_type: Value,
    pub created_ts: Value,
    pub payload: Value,
}

impl Into<InternalEvent> for Event {
    fn into(self) -> InternalEvent {
        InternalEvent {
            event_type: Value::Text(self.event_type),
            created_ts: Value::Text(self.created_ts),
            payload: Value::Map(self.payload),
        }
    }
}

impl Into<Value> for InternalEvent {
    fn into(self) -> Value {
        let mut payload = Payload::new();
        payload.insert("type".to_owned(), self.event_type);
        payload.insert("created_ts".to_owned(), self.created_ts);
        payload.insert("payload".to_owned(), self.payload);
        Value::Map(payload)
    }
}

/// A ProcessedEvent is the result of the matcher process.
/// It contains the original Event along with the result of the matching operation.
#[derive(Debug, Clone)]
pub struct ProcessedEvent {
    pub event: InternalEvent,
    pub rules: HashMap<String, ProcessedRule>,
    pub extracted_vars: HashMap<String, Value>,
}

impl ProcessedEvent {
    pub fn new(event: Event) -> ProcessedEvent {
        ProcessedEvent {
            event: event.into(),
            rules: HashMap::new(),
            extracted_vars: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessedRule {
    pub rule_name: String,
    pub status: ProcessedRuleStatus,
    pub actions: Vec<Action>,
    pub message: Option<String>,
}

impl ProcessedRule {
    pub fn new(rule_name: String) -> ProcessedRule {
        ProcessedRule {
            rule_name,
            status: ProcessedRuleStatus::NotProcessed,
            actions: vec![],
            message: None,
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
