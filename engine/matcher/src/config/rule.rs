//! The config module contains the *struct* definitions required for configuring the matcher.
//! For example, it contains the definition of the Rule and Filter structs and the mapping to
//! serialize/deserialize them to/from json format.

use crate::error::MatcherError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tornado_common_api::{Payload, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Constraint {
    #[serde(rename = "WHERE")]
    pub where_operator: Option<Operator>,
    #[serde(rename = "WITH")]
    pub with: HashMap<String, Extractor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Extractor {
    pub from: String,
    pub regex: ExtractorRegex,
    #[serde(default)]
    pub modifiers_post: Vec<Modifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
pub enum Modifier {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum ExtractorRegex {
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
    SingleKeyRegex {
        #[serde(rename = "single_key_match")]
        regex: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
pub enum Operator {
    #[serde(rename = "AND")]
    And { operators: Vec<Operator> },
    #[serde(rename = "OR")]
    Or { operators: Vec<Operator> },
    #[serde(rename = "NOT")]
    Not { operator: Box<Operator> },
    #[serde(rename = "contains")]
    #[serde(alias = "contain")]
    Contains { first: Value, second: Value },
    #[serde(rename = "containsIgnoreCase")]
    #[serde(alias = "containIgnoreCase")]
    ContainsIgnoreCase { first: Value, second: Value },
    #[serde(rename = "equals")]
    #[serde(alias = "equal")]
    Equals { first: Value, second: Value },
    #[serde(rename = "equalsIgnoreCase")]
    #[serde(alias = "equalIgnoreCase")]
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
    #[serde(alias = "notEquals")]
    #[serde(alias = "notEqual")]
    NotEquals { first: Value, second: Value },
    #[serde(rename = "regex")]
    Regex { regex: String, target: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Action {
    pub id: String,
    pub payload: Payload,
}

impl From<Action> for Value {
    fn from(action: Action) -> Self {
        let mut new_value = HashMap::new();
        new_value.insert("id".to_owned(), Value::Text(action.id));
        new_value.insert("payload".to_owned(), Value::Map(action.payload));
        Value::Map(new_value)
    }
}

impl Rule {
    pub fn from_json(json: &str) -> Result<Rule, MatcherError> {
        serde_json::from_str(json).map_err(|e| MatcherError::JsonDeserializationError {
            message: format!("Cannot deserialize Rule. Error [{}]", e),
        })
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::fs;

    #[test]
    fn should_return_error_if_invalid_json() {
        let json = r#"{"hello":"world"}"#;
        let rule = Rule::from_json(&json);
        assert!(rule.is_err())
    }

    #[test]
    fn should_deserialize_rule_from_json() {
        let json = file_to_string("./test_resources/rules/001_all_emails_and_syslogs.json");
        let rule = Rule::from_json(&json).unwrap();

        assert_eq!("", rule.name);

        match rule.constraint.where_operator.unwrap() {
            Operator::And { operators } => {
                assert_eq!(2, operators.len());
            }
            _ => assert!(false),
        }

        let extractor1 = &rule.constraint.with["extracted_temp"];
        assert_eq!("${event.payload.body}", extractor1.from);
        match &extractor1.regex {
            ExtractorRegex::Regex { regex, all_matches, group_match_idx } => {
                assert_eq!("([0-9]+\\sDegrees)", regex);
                assert_eq!(&Some(2), group_match_idx);
                assert_eq!(all_matches, &None);
            }
            _ => assert!(false),
        }

        let extractor2 = &rule.constraint.with["all_temperatures"];
        assert_eq!("${event.payload.body}", extractor1.from);
        match &extractor2.regex {
            ExtractorRegex::Regex { regex, group_match_idx, all_matches } => {
                assert_eq!("([0-9]+\\sDegrees)", regex);
                assert_eq!(&None, group_match_idx);
                assert_eq!(all_matches, &Some(true));
            }
            _ => assert!(false),
        }

        let extractor2 = &rule.constraint.with["all_temperatures_named"];
        assert_eq!("${event.payload.body}", extractor1.from);
        match &extractor2.regex {
            ExtractorRegex::RegexNamedGroups { regex, all_matches } => {
                assert_eq!("(?P<DEGREES>[0-9]+\\sDegrees)", regex);
                assert_eq!(all_matches, &None);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_deserialize_rule_without_where_from_json() {
        // Arrange
        let json = file_to_string("./test_resources/rules/002_rule_without_where.json");

        // Act
        let rule = Rule::from_json(&json).unwrap();

        // Assert
        assert!(rule.constraint.where_operator.is_none())
    }

    fn file_to_string(filename: &str) -> String {
        fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename))
    }

    #[test]
    fn should_deserialize_rule_from_json_with_map_in_action_payload() {
        let json = file_to_string("./test_resources/rules/003_map_in_action_payload.json");
        let rule = Rule::from_json(&json).unwrap();

        assert_eq!("", rule.name);

        match rule.constraint.where_operator.unwrap() {
            Operator::And { operators } => {
                assert_eq!(1, operators.len());
            }
            _ => assert!(false),
        }

        let extractor1 = &rule.constraint.with["extracted_temp"];
        assert_eq!("${event.payload.body}", extractor1.from);
        match &extractor1.regex {
            ExtractorRegex::Regex { regex, all_matches, group_match_idx: _ } => {
                assert_eq!("([0-9]+\\sDegrees)", regex);
                assert_eq!(all_matches, &None);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_deserialize_rule_from_json_with_cmp_operators() {
        let json = file_to_string("./test_resources/rules/004_cmp_operators.json");
        let rule = Rule::from_json(&json);

        assert!(rule.is_ok());
    }

    #[test]
    fn should_deserialize_rule_from_json_with_single_key_match() {
        let json = file_to_string("./test_resources/rules/006_with_single_key_match.json");
        let rule = Rule::from_json(&json);

        assert!(rule.is_ok());
    }

    #[test]
    fn should_deserialize_rule_from_json_with_modifiers_post() {
        let json = file_to_string("./test_resources/rules/007_with_modifiers_post.json");
        let rule = Rule::from_json(&json);
        assert!(rule.is_ok());
    }
}
