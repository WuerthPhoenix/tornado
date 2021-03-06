use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use typescript_definitions::TypeScriptify;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
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

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct ConstraintDto {
    #[serde(rename = "WHERE")]
    pub where_operator: Option<OperatorDto>,
    #[serde(rename = "WITH")]
    pub with: HashMap<String, ExtractorDto>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct ExtractorDto {
    pub from: String,
    pub regex: ExtractorRegexDto,
    #[serde(default)]
    pub modifiers_post: Vec<ModifierDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TypeScriptify)]
#[serde(tag = "type")]
pub enum ModifierDto {
    Lowercase {},
    Map {
        mapping: HashMap<String, String>,
        default_value: Option<String>,
    },
    ReplaceAll {
        find: String,
        replace: String,
        #[serde(default)]
        is_regex: bool,
    },
    ToNumber {},
    Trim {},
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
#[serde(tag = "type")]
pub enum ExtractorRegexDto {
    Regex {
        #[serde(rename = "match")]
        regex: String,
        group_match_idx: Option<usize>,
        all_matches: Option<bool>,
    },
    RegexNamedGroups {
        #[serde(rename = "named_match")]
        regex: String,
        all_matches: Option<bool>,
    },
    KeyRegex {
        #[serde(rename = "single_key_match")]
        regex: String,
    },
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
#[serde(tag = "type")]
pub enum OperatorDto {
    #[serde(rename = "AND")]
    And { operators: Vec<OperatorDto> },
    #[serde(rename = "OR")]
    Or { operators: Vec<OperatorDto> },
    #[serde(rename = "NOT")]
    Not { operator: Box<OperatorDto> },
    #[serde(rename = "contains")]
    Contains { first: Value, second: Value },
    #[serde(rename = "containsIgnoreCase")]
    ContainsIgnoreCase { first: Value, second: Value },
    #[serde(rename = "equals")]
    Equals { first: Value, second: Value },
    #[serde(rename = "equalsIgnoreCase")]
    EqualsIgnoreCase { first: Value, second: Value },
    #[serde(rename = "ge")]
    GreaterEqualThan { first: Value, second: Value },
    #[serde(rename = "gt")]
    GreaterThan { first: Value, second: Value },
    #[serde(rename = "le")]
    LessEqualThan { first: Value, second: Value },
    #[serde(rename = "lt")]
    LessThan { first: Value, second: Value },
    #[serde(rename = "ne")]
    NotEquals { first: Value, second: Value },
    #[serde(rename = "regex")]
    Regex { regex: String, target: String },
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct ActionDto {
    pub id: String,
    pub payload: Value,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct FilterDto {
    pub description: String,
    pub active: bool,
    pub filter: Option<OperatorDto>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
#[serde(tag = "type")]
pub enum MatcherConfigDto {
    Filter { name: String, filter: FilterDto, nodes: Vec<MatcherConfigDto> },
    Ruleset { name: String, rules: Vec<RuleDto> },
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct MatcherConfigDraftDto {
    pub data: MatcherConfigDraftDataDto,
    pub config: MatcherConfigDto,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct MatcherConfigDraftDataDto {
    pub user: String,
    pub created_ts_ms: i64,
    pub updated_ts_ms: i64,
    pub draft_id: String,
}
