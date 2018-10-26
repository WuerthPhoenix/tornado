use std::collections::HashMap;
use tornado_common_api::{Action, Event};

/// The ProcessedEvent is the result of the matcher process.
/// It contains the original Event along with the result of the matching operation.
#[derive(Debug, Clone)]
pub struct ProcessedEvent<'o> {
    pub event: Event,
    pub matched: HashMap<&'o str, ProcessedRule<'o>>
}

impl<'o> ProcessedEvent<'o> {
    pub fn new(event: Event) -> ProcessedEvent<'o> {
        ProcessedEvent {
            event,
            matched: HashMap::new()
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessedRule<'o> {
    pub status: ProcessedRuleStatus,
    pub extracted_vars: HashMap<&'o str, String>,
    pub actions: Vec<Action>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessedRuleStatus {
    Matched,
    PartiallyMatched,
    NotMatched,
    NotProcessed
}