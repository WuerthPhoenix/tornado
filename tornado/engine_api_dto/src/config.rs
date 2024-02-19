use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::iter::Sum;
use std::ops::Add;
use tornado_engine_matcher::config::filter::Filter;
use tornado_engine_matcher::config::rule::{Operator, Rule};
use tornado_engine_matcher::config::{Defaultable, MatcherConfig};
use typescript_definitions::TypeScriptify;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct RuleDetailsDto {
    #[serde(default)]
    pub name: String,
    pub description: String,
    #[serde(rename = "continue")]
    pub do_continue: bool,
    pub active: bool,
    pub actions: Vec<String>,
}

impl From<&Rule> for RuleDetailsDto {
    fn from(rule: &Rule) -> Self {
        RuleDetailsDto {
            name: rule.name.to_owned(),
            description: rule.description.to_owned(),
            do_continue: rule.do_continue,
            active: rule.active,
            actions: rule.actions.iter().map(|action| action.to_owned().id).collect(),
        }
    }
}

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
pub struct RulePositionDto {
    pub position: usize,
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
    DateAndTime {
        timezone: String,
    },
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

impl From<&Operator> for OperatorDto {
    fn from(operator: &Operator) -> Self {
        match operator {
            Operator::And { operators } => {
                OperatorDto::And { operators: operators.iter().map(OperatorDto::from).collect() }
            }
            Operator::Or { operators } => {
                OperatorDto::Or { operators: operators.iter().map(OperatorDto::from).collect() }
            }
            Operator::Not { operator } => {
                OperatorDto::Not { operator: Box::new(OperatorDto::from(operator.as_ref())) }
            }
            Operator::Contains { first, second } => OperatorDto::Contains {
                first: serde_json::to_value(first).unwrap_or(serde_json::Value::Null),
                second: serde_json::to_value(second).unwrap_or(serde_json::Value::Null),
            },
            Operator::ContainsIgnoreCase { first, second } => OperatorDto::ContainsIgnoreCase {
                first: serde_json::to_value(first).unwrap_or(serde_json::Value::Null),
                second: serde_json::to_value(second).unwrap_or(serde_json::Value::Null),
            },
            Operator::Equals { first, second } => OperatorDto::Equals {
                first: serde_json::to_value(first).unwrap_or(serde_json::Value::Null),
                second: serde_json::to_value(second).unwrap_or(serde_json::Value::Null),
            },
            Operator::EqualsIgnoreCase { first, second } => OperatorDto::EqualsIgnoreCase {
                first: serde_json::to_value(first).unwrap_or(serde_json::Value::Null),
                second: serde_json::to_value(second).unwrap_or(serde_json::Value::Null),
            },
            Operator::GreaterEqualThan { first, second } => OperatorDto::GreaterEqualThan {
                first: serde_json::to_value(first).unwrap_or(serde_json::Value::Null),
                second: serde_json::to_value(second).unwrap_or(serde_json::Value::Null),
            },
            Operator::GreaterThan { first, second } => OperatorDto::GreaterThan {
                first: serde_json::to_value(first).unwrap_or(serde_json::Value::Null),
                second: serde_json::to_value(second).unwrap_or(serde_json::Value::Null),
            },
            Operator::LessEqualThan { first, second } => OperatorDto::LessEqualThan {
                first: serde_json::to_value(first).unwrap_or(serde_json::Value::Null),
                second: serde_json::to_value(second).unwrap_or(serde_json::Value::Null),
            },
            Operator::LessThan { first, second } => OperatorDto::LessThan {
                first: serde_json::to_value(first).unwrap_or(serde_json::Value::Null),
                second: serde_json::to_value(second).unwrap_or(serde_json::Value::Null),
            },
            Operator::NotEquals { first, second } => OperatorDto::NotEquals {
                first: serde_json::to_value(first).unwrap_or(serde_json::Value::Null),
                second: serde_json::to_value(second).unwrap_or(serde_json::Value::Null),
            },
            Operator::Regex { regex, target } => {
                OperatorDto::Regex { regex: regex.to_owned(), target: target.to_owned() }
            }
        }
    }
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
impl From<Filter> for FilterDto {
    fn from(filter: Filter) -> Self {
        FilterDto {
            description: filter.description.to_owned(),
            filter: match &filter.filter {
                Defaultable::Value(operator) => Some(operator.into()),
                Defaultable::Default { .. } => None,
            },
            active: filter.active,
        }
    }
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

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
#[serde(tag = "type")]
pub enum ProcessingTreeNodeConfigDto {
    Filter {
        name: String,
        rules_count: usize,
        children_count: usize,
        description: String,
        active: bool,
    },
    Ruleset {
        name: String,
        rules_count: usize,
    },
}

impl From<&MatcherConfig> for ProcessingTreeNodeConfigDto {
    fn from(matcher_config_node: &MatcherConfig) -> Self {
        match matcher_config_node {
            MatcherConfig::Filter { name, filter, .. } => ProcessingTreeNodeConfigDto::Filter {
                name: name.to_owned(),
                rules_count: matcher_config_node.get_all_rules_count(),
                children_count: matcher_config_node.get_direct_child_nodes_count(),
                description: filter.to_owned().description,
                active: filter.active,
            },

            MatcherConfig::Ruleset { name, .. } => ProcessingTreeNodeConfigDto::Ruleset {
                name: name.to_owned(),
                rules_count: matcher_config_node.get_all_rules_count(),
            },
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
#[serde(tag = "type")]
pub enum ProcessingTreeNodeDetailsDto {
    Filter { name: String, description: String, active: bool, filter: Option<OperatorDto> },
    Ruleset { name: String, rules: Vec<RuleDetailsDto> },
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
#[serde(tag = "type")]
pub enum ProcessingTreeNodeEditDto {
    Filter { name: String, description: String, active: bool, filter: Option<OperatorDto> },
    Ruleset { name: String },
}

impl From<&MatcherConfig> for ProcessingTreeNodeDetailsDto {
    fn from(matcher_config_node: &MatcherConfig) -> Self {
        match matcher_config_node {
            MatcherConfig::Filter { name, filter, .. } => ProcessingTreeNodeDetailsDto::Filter {
                name: name.to_owned(),
                description: filter.to_owned().description,
                active: filter.clone().active,
                filter: match &filter.filter {
                    Defaultable::Value(operator) => Some(operator.into()),
                    Defaultable::Default { .. } => None,
                },
            },

            MatcherConfig::Ruleset { name, rules, .. } => {
                let rules_details_dto = rules.iter().map(RuleDetailsDto::from).collect();
                ProcessingTreeNodeDetailsDto::Ruleset {
                    name: name.to_owned(),
                    rules: rules_details_dto,
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize, TypeScriptify)]
pub struct TreeInfoDto {
    pub rules_count: usize,
    pub filters_count: usize,
}

impl Add for TreeInfoDto {
    type Output = TreeInfoDto;

    fn add(self, rhs: Self) -> Self::Output {
        TreeInfoDto {
            filters_count: self.filters_count + rhs.filters_count,
            rules_count: self.rules_count + rhs.rules_count,
        }
    }
}

impl Sum for TreeInfoDto {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(TreeInfoDto::default(), Add::add)
    }
}
