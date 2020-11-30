use tornado_common_api::{Action, Event, Number, Payload, Value};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

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
    pub meta: Option<ProcessedRuleMetaData>
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessedRuleMetaData {
    pub actions: Vec<ActionMetaData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionMetaData {
    pub id: String,
    pub payload: PayloadMetaData,
}

pub type PayloadMetaData = HashMap<String, EnrichedValue>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum EnrichedValue {
    Single(ValueContent<Value>),
    Map(ValueContent<PayloadMetaData>),
    Array(ValueContent<Vec<EnrichedValue>>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValueContent<T> {
    pub content: T,
    pub meta: ValueMetaData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValueMetaData {
    //pub original: Option<Value>,
    pub modified: bool,
    pub is_leaf: bool,
}