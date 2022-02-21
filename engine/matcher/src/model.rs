use crate::accessor::{EVENT_KEY, EXTRACTED_VARIABLES_KEY, FOREACH_ITEM_KEY};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tornado_common_api::{Action, ValueGet};
use typescript_definitions::TypeScriptify;

pub struct InternalEvent<'o> {
    pub event: &'o Value,
    pub extracted_variables: &'o mut Value,
}

impl<'o> From<(&'o Value, &'o mut Value)> for InternalEvent<'o> {
    fn from((event, extracted_variables): (&'o Value, &'o mut Value)) -> Self {
        Self { event, extracted_variables }
    }
}

impl<'o> ValueGet for InternalEvent<'o> {
    fn get_from_map(&self, key: &str) -> Option<&tornado_common_api::Value> {
        match key {
            EVENT_KEY => Some(self.event),
            EXTRACTED_VARIABLES_KEY => Some(self.extracted_variables),
            FOREACH_ITEM_KEY => Some(self.event),
            _ => None,
        }
    }

    fn get_from_array(&self, _index: usize) -> Option<&tornado_common_api::Value> {
        None
    }
}

/// A ProcessedEvent is the result of the matcher process.
/// It contains the original Event along with the result of the matching operation.
#[derive(Debug, Clone)]
pub struct ProcessedEvent {
    pub event: Value,
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
