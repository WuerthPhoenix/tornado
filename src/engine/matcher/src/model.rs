use std::collections::HashMap;
use tornado_common_api::{Action, Event, Number, Payload, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct InternalEvent {
    pub event_type: Value,
    pub created_ms: Value,
    pub payload: Value,
}

impl Into<InternalEvent> for Event {
    fn into(self) -> InternalEvent {
        InternalEvent {
            event_type: Value::Text(self.event_type),
            created_ms: Value::Number(Number::PosInt(self.created_ms)),
            payload: Value::Map(self.payload),
        }
    }
}

impl Into<Value> for InternalEvent {
    fn into(self) -> Value {
        let mut payload = Payload::new();
        payload.insert("type".to_owned(), self.event_type);
        payload.insert("created_ms".to_owned(), self.created_ms);
        payload.insert("payload".to_owned(), self.payload);
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
    Filter { name: String, filter: ProcessedFilter, nodes: HashMap<String, ProcessedNode> },
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
    pub rules: HashMap<String, ProcessedRule>,
    pub extracted_vars: HashMap<String, Value>,
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
