use error::MatcherError;
use serde_json;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub description: String,
    pub priority: u16,
    #[serde(rename = "continue")]
    pub do_continue: bool,
    pub active: bool,
    pub constraint: Constraint,
    pub actions: Vec<Action>,
}

impl Rule {
    pub fn from_json(json: &str) -> Result<Rule, MatcherError> {
        serde_json::from_str(&json).map_err(|e| MatcherError::JsonDeserializationError {
            message: format!("Cannot deserialize Rule. Error [{}]", e),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    #[serde(rename = "WHERE")]
    pub where_operator: Operator,
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
    #[serde(rename = "equal")]
    Equal { first: String, second: String },
    #[serde(rename = "regex")]
    Regex { regex: String, target: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub payload: HashMap<String, String>,
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

        match rule.constraint.where_operator {
            Operator::And { operators } => {
                assert_eq!(2, operators.len());
            }
            _ => assert!(false),
        }

        assert_eq!("${event.payload.body}", rule.constraint.with["extracted_temp"].from);
        assert_eq!("([0-9]+\\sDegrees)", rule.constraint.with["extracted_temp"].regex.regex);
    }

    fn file_to_string(filename: &str) -> String {
        fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename))
    }

}
