//! The config module contains the *struct* definitions required for configuring the matcher.
//! For example, it contains the definition of the Rule struct and its mapping to
//! serialize/deserialize it to/from json format.

use crate::error::MatcherError;
use log::{info, trace};
use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs;
use tornado_common_api::Payload;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn read_rules_from_dir_sorted_by_filename(dir: &str) -> Result<Vec<Rule>, MatcherError> {
        let mut paths = fs::read_dir(dir)
            .and_then(|entry_set| entry_set.collect::<Result<Vec<_>, _>>())
            .map_err(|e| MatcherError::ConfigurationError {
                message: format!("Error reading from config path [{}]: {}", dir, e),
            })?;

        // Sort by filename
        paths.sort_by_key(|dir| dir.path());

        let mut rules = vec![];

        for entry in paths {
            let path = entry.path();

            let filename = path.to_str().ok_or_else(|| MatcherError::ConfigurationError {
                message: format!("Error processing filename of file: [{}]", path.display()),
            })?;

            if !filename.ends_with(".json") {
                info!("Configuration file [{}] is ignored.", path.display());
                continue;
            }

            info!("Loading rule from file: [{}]", path.display());
            let rule_body =
                fs::read_to_string(&path).map_err(|e| MatcherError::ConfigurationError {
                    message: format!("Unable to open the file [{}]. Err: {}", path.display(), e),
                })?;

            trace!("Rule body: \n{}", rule_body);
            rules.push(Rule::from_json(&rule_body)?)
        }

        info!("Loaded {} rule(s) from [{}]", rules.len(), dir);

        Ok(rules)
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
        let json = file_to_string("./test_resources/rules/rule_01.json");
        let rule = Rule::from_json(&json).unwrap();

        assert_eq!("all_emails_and_syslogs", rule.name);

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
        let json = file_to_string("./test_resources/rules/rule_02_no_where.json");

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
        let json = file_to_string("./test_resources/rules/rule_03_map_in_action_payload.json");
        let rule = Rule::from_json(&json).unwrap();

        assert_eq!("map_in_action_payload", rule.name);

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
    fn should_read_from_folder_sorting_by_filename() {
        let path = "./test_resources/rules";
        let rules = Rule::read_rules_from_dir_sorted_by_filename(path).unwrap();

        assert_eq!(3, rules.len());

        assert_eq!("all_emails_and_syslogs", rules.get(0).unwrap().name);
        assert_eq!("rule_without_where", rules.get(1).unwrap().name);
        assert_eq!("map_in_action_payload", rules.get(2).unwrap().name);
    }
}
