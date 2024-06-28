use crate::config::rule::Operator;
use crate::config::Defaultable;
use crate::error::MatcherError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    pub description: String,
    pub active: bool,
    pub filter: Defaultable<Operator>,
}

impl Filter {
    pub fn from_json(json: &str) -> Result<Filter, MatcherError> {
        serde_json::from_str(json).map_err(|e| MatcherError::JsonDeserializationError {
            message: format!("Cannot deserialize Filter. Error [{}]", e),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct MatcherIterator {
    pub(crate) description: String,
    pub(crate) active: bool,
    pub(crate) target: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use tornado_common_api::Value;

    #[test]
    fn should_deserialize_filter_from_json() {
        let filename = "./test_resources/v1/filter/filter_01.json";
        let json = fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename));

        let filter = Filter::from_json(&json).unwrap();

        assert_eq!(
            Defaultable::Value(Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned())
            }),
            filter.filter
        );
    }

    #[test]
    fn should_deserialize_with_empty_filter_type_field() {
        let json = r##"{
          "description": "This filter allows only events with type email",
          "active": true,
          "filter": {}
        }"##;

        let filter = Filter::from_json(json).unwrap();

        assert_eq!(Defaultable::Default {}, filter.filter);
    }

    #[test]
    fn should_not_deserialize_with_unknown_field() {
        let json = r##"{
          "description": "This filter allows only events with type email",
          "active": true,
          "filter": {},
          "constraint": {}
        }"##;

        assert!(Filter::from_json(json).is_err());
    }

    #[test]
    fn should_not_deserialize_with_missing_filter_field() {
        let json = r##"{
          "description": "This filter allows only events with type email",
          "active": true
        }"##;

        assert!(Filter::from_json(json).is_err());
    }
}
