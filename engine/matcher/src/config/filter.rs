use crate::config::rule::Operator;
use crate::error::MatcherError;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub description: String,
    pub active: bool,
    pub filter: Operator,
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

        assert_eq!(
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned())
            },
            filter.filter
        );
    }

    #[test]
    fn should_deserialize_with_empty_filter_field() {

        let json = r##"
        {
          "description": "This filter allows only events with type email",
          "active": true,
          "filter": {}
        }
        "##;

        let filter = Filter::from_json(&json).unwrap();

        assert_eq!(
            Operator::None,
            filter.filter
        );
    }

    #[test]
    fn should_not_deserialize_with_missing_filter_field() {

        let json = r##"
        {
          "description": "This filter allows only events with type email",
          "active": true
        }
        "##;

        assert!(Filter::from_json(&json).is_err());

    }
}
