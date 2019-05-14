#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::btree_map::BTreeMap;
use std::collections::HashMap;
use typescript_definitions::TypescriptDefinition;

#[derive(Clone, Serialize, Deserialize, TypescriptDefinition)]
pub struct SendEventRequestDto {
    pub process_type: ProcessType,
    pub event: EventDto,
}

#[derive(Clone, Serialize, Deserialize, TypescriptDefinition)]
pub enum ProcessType {
    Full,
    SkipActions,
}

#[derive(Clone, Serialize, Deserialize, TypescriptDefinition)]
pub struct EventDto {
    #[serde(rename = "type")]
    pub event_type: String,
    pub created_ms: u64,
    pub payload: HashMap<String, Value>,
}

#[derive(Clone, Serialize, Deserialize, TypescriptDefinition)]
pub struct ProcessedEventDto {
    pub event: EventDto,
    pub result: ProcessedNodeDto,
}

#[derive(Clone, Serialize, Deserialize, TypescriptDefinition)]
#[serde(tag = "type")]
pub enum ProcessedNodeDto {
    Filter { filter: ProcessedFilterDto, nodes: BTreeMap<String, ProcessedNodeDto> },
    Rules { rules: ProcessedRulesDto },
}

#[derive(Clone, Serialize, Deserialize, TypescriptDefinition)]
pub struct ProcessedFilterDto {
    pub name: String,
    pub status: ProcessedFilterStatusDto,
}

#[derive(Clone, Serialize, Deserialize, TypescriptDefinition)]
pub enum ProcessedFilterStatusDto {
    Matched,
    NotMatched,
    Inactive,
}

#[derive(Clone, Serialize, Deserialize, TypescriptDefinition)]
pub struct ProcessedRulesDto {
    pub rules: HashMap<String, ProcessedRuleDto>,
    pub extracted_vars: HashMap<String, Value>,
}

#[derive(Clone, Serialize, Deserialize, TypescriptDefinition)]
pub struct ProcessedRuleDto {
    pub rule_name: String,
    pub status: ProcessedRuleStatusDto,
    pub actions: Vec<ActionDto>,
    pub message: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, TypescriptDefinition)]
pub enum ProcessedRuleStatusDto {
    Matched,
    PartiallyMatched,
    NotMatched,
    NotProcessed,
}

#[derive(Clone, Serialize, Deserialize, TypescriptDefinition)]
pub struct ActionDto {
    pub id: String,
    pub payload: Value,
}
