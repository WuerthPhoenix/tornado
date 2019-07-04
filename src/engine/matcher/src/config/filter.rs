use crate::config::rule::Operator;
use crate::error::MatcherError;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    #[serde(default)]
    pub name: String,
    pub description: String,
    pub active: bool,
    pub filter: Option<Operator>,
}

impl Filter {
    pub fn from_json(json: &str) -> Result<Filter, MatcherError> {
        serde_json::from_str(&json).map_err(|e| MatcherError::JsonDeserializationError {
            message: format!("Cannot deserialize Filter. Error [{}]", e),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use tornado_common_api::Value;

    #[test]
    fn should_deserialize_filter_from_json() {
        let filename = "./test_resources/filter/filter_01.json";
        let json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

        let filter = Filter::from_json(&json).unwrap();

        assert_eq!("only_emails", filter.name);

        assert_eq!(
            Some(Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned())
            }),
            filter.filter
        );
    }
}
