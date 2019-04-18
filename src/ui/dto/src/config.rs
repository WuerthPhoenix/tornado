#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::collections::btree_map::BTreeMap;
use typescript_definitions::TypescriptDefinition;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct Rule {
    #[serde(default)]
    pub name: String,
    pub description: String,
    #[serde(rename = "continue")]
    pub do_continue: bool,
    pub active: bool,
    pub constraint: Constraint,
    pub actions: Vec<Action>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct Constraint {
    #[serde(rename = "WHERE")]
    pub where_operator: Option<Operator>,
    #[serde(rename = "WITH")]
    pub with: HashMap<String, Extractor>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct Extractor {
    pub from: String,
    pub regex: ExtractorRegex,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct ExtractorRegex {
    #[serde(rename = "match")]
    pub regex: String,
    pub group_match_idx: u16,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition)]
#[serde(tag = "type")]
pub enum Operator {
    #[serde(rename = "AND")]
    And { operators: Vec<Operator> },
    #[serde(rename = "OR")]
    Or { operators: Vec<Operator> },
    #[serde(rename = "contain")]
    Contain { text: String, substring: String },
    #[serde(rename = "equal")]
    Equal { first: String, second: String },
    #[serde(rename = "regex")]
    Regex { regex: String, target: String },
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct Action {
    pub id: String,
    pub payload: Value,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct Filter {
    #[serde(default)]
    pub name: String,
    pub description: String,
    pub active: bool,
    pub filter: Option<Operator>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypescriptDefinition)]
pub enum MatcherConfig {
    Filter { filter: Filter, nodes: BTreeMap<String, MatcherConfig> },
    Rules { rules: Vec<Rule> },
}