use std::collections::HashMap;
use tornado_common_api::{Action, Event, Value};

/// The ProcessedEvent is the result of the matcher process.
/// It contains the original Event along with the result of the matching operation.
#[derive(Debug, Clone)]
pub struct ProcessedEvent {
    pub event: Value,
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
