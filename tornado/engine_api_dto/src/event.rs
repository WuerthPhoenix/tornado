use crate::config::ActionDto;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tornado_engine_matcher::model::ProcessedRuleMetaData;
use typescript_definitions::TypeScriptify;

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub struct SendEventRequestDto {
    pub process_type: ProcessType,
    pub event: EventDto,
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub enum ProcessType {
    Full,
    SkipActions,
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub struct EventDto {
    #[serde(rename = "type")]
    pub event_type: String,
    pub created_ms: u64,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    pub payload: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterator: Option<EventIteratorDataDto>
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub struct EventIteratorDataDto {
    pub item: Value,
    pub iteration: StringOrInt,
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
#[serde(untagged)]
pub enum StringOrInt {
    String(String),
    Int(u64),
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub struct ProcessedEventDto {
    pub event: EventDto,
    pub result: ProcessedNodeDto,
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
#[serde(tag = "type")]
pub enum ProcessedNodeDto {
    Filter { name: String, filter: ProcessedFilterDto, nodes: Vec<ProcessedNodeDto> },
    Iterator { name: String, iterator: ProcessedIteratorDto, events: Vec<ProcessedIterationDto> },
    Ruleset { name: String, rules: ProcessedRulesDto },
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub struct ProcessedIteratorDto {
    pub status: ProcessedIteratorStatusDto,
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub enum ProcessedIteratorStatusDto {
    Matched,
    AccessorError,
    TypeError,
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub struct ProcessedIterationDto {
    pub event: EventDto,
    pub nodes: Vec<ProcessedNodeDto>,
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub struct ProcessedFilterDto {
    pub status: ProcessedFilterStatusDto,
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub enum ProcessedFilterStatusDto {
    Matched,
    NotMatched,
    Inactive,
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub struct ProcessedRulesDto {
    pub rules: Vec<ProcessedRuleDto>,
    pub extracted_vars: Value,
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub struct ProcessedRuleDto {
    pub name: String,
    pub status: ProcessedRuleStatusDto,
    pub actions: Vec<ActionDto>,
    pub message: Option<String>,
    pub meta: Option<ProcessedRuleMetaData>,
}

#[derive(Clone, Serialize, Deserialize, TypeScriptify)]
pub enum ProcessedRuleStatusDto {
    Matched,
    PartiallyMatched,
    NotMatched,
    NotProcessed,
}
