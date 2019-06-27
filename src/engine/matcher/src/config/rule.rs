//! The config module contains the *struct* definitions required for configuring the matcher.
//! For example, it contains the definition of the Rule and Filter structs and the mapping to
//! serialize/deserialize them to/from json format.

use crate::error::MatcherError;
use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use tornado_common_api::{Payload, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    #[serde(rename = "WHERE")]
    pub where_operator: Option<Operator>,
    #[serde(rename = "WITH")]
    pub with: HashMap<String, Extractor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extractor {
    pub from: String,
    pub regex: ExtractorRegex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorRegex {
    #[serde(rename = "match")]
    pub regex: String,
    pub group_match_idx: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Operator {
    #[serde(rename = "AND")]
    And { operators: Vec<Operator> },
    #[serde(rename = "OR")]
    Or { operators: Vec<Operator> },
    #[serde(rename = "contain")]
    Contain { text: String, substring: String },
    #[serde(rename = "equal")]
    Equal { first: Value, second: Value },
    #[serde(rename = "ge")]
    GreaterEqualThan { first: Value, second: Value },
    #[serde(rename = "gt")]
    GreaterThan { first: Value, second: Value },
    #[serde(rename = "le")]
    LessEqualThan { first: Value, second: Value },
    #[serde(rename = "lt")]
    LessThan { first: Value, second: Value },
    #[serde(rename = "regex")]
    Regex { regex: String, target: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub payload: Payload,
}

impl Rule {
    pub fn from_json(json: &str) -> Result<Rule, MatcherError> {
        serde_json::from_str(&json).map_err(|e| MatcherError::JsonDeserializationError {
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

        assert_eq!("${event.payload.body}", rule.constraint.with["extracted_temp"].from);
        assert_eq!("([0-9]+\\sDegrees)", rule.constraint.with["extracted_temp"].regex.regex);
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

        assert_eq!("${event.payload.body}", rule.constraint.with["extracted_temp"].from);
        assert_eq!("([0-9]+\\sDegrees)", rule.constraint.with["extracted_temp"].regex.regex);
    }

    #[test]
    fn should_deserialize_rule_from_json_with_cmp_operators() {
        let json = file_to_string("./test_resources/rules/004_cmp_operators.json");
        let rule = Rule::from_json(&json);

        assert!(rule.is_ok());
    }

}
