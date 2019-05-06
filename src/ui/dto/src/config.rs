#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::btree_map::BTreeMap;
use std::collections::HashMap;
use typescript_definitions::TypescriptDefinition;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct RuleDto {
    #[serde(default)]
    pub name: String,
    pub description: String,
    #[serde(rename = "continue")]
    pub do_continue: bool,
    pub active: bool,
    pub constraint: ConstraintDto,
    pub actions: Vec<ActionDto>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct ConstraintDto {
    #[serde(rename = "WHERE")]
    pub where_operator: Option<OperatorDto>,
    #[serde(rename = "WITH")]
    pub with: HashMap<String, ExtractorDto>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct ExtractorDto {
    pub from: String,
    pub regex: ExtractorRegexDto,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct ExtractorRegexDto {
    #[serde(rename = "match")]
    pub regex: String,
    pub group_match_idx: u16,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition)]
#[serde(tag = "type")]
pub enum OperatorDto {
    #[serde(rename = "AND")]
    And { operators: Vec<OperatorDto> },
    #[serde(rename = "OR")]
    Or { operators: Vec<OperatorDto> },
    #[serde(rename = "contain")]
    Contain { text: String, substring: String },
    #[serde(rename = "equal")]
    Equal { first: String, second: String },
    #[serde(rename = "regex")]
    Regex { regex: String, target: String },
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct ActionDto {
    pub id: String,
    pub payload: Value,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct FilterDto {
    #[serde(default)]
    pub name: String,
    pub description: String,
    pub active: bool,
    pub filter: Option<OperatorDto>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition)]
#[serde(tag = "type")]
pub enum MatcherConfigDto {
    Filter { filter: FilterDto, nodes: BTreeMap<String, MatcherConfigDto> },
    Rules { rules: Vec<RuleDto> },
}
