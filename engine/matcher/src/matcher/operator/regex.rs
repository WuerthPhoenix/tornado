use crate::error::MatcherError;
use crate::matcher::operator::Operator;
use crate::{accessor::Accessor, model::InternalEvent};
use regex::Regex as RustRegex;
use tornado_common_api::cow_to_str;

const OPERATOR_NAME: &str = "regex";

/// A matching matcher.operator that checks whether a string matches a given regex
#[derive(Debug)]
pub struct Regex {
    regex: RustRegex,
    target: Accessor,
}

impl Regex {
    pub fn build(regex: &str, target: Accessor) -> Result<Regex, MatcherError> {
        let regex = RustRegex::new(regex).map_err(|e| MatcherError::OperatorBuildFailError {
            message: format!("Cannot parse regex [{}]", regex),
            cause: e.to_string(),
        })?;

        Ok(Regex { target, regex })
    }
}

impl Operator for Regex {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &InternalEvent) -> bool {
        let cow_value = self.target.get(event);
        cow_to_str(&cow_value).is_some_and(|text| self.regex.is_match(text))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::accessor::AccessorBuilder;
    use serde_json::json;
    use tornado_common_api::*;

    #[test]
    fn should_return_the_operator_name() {
        let operator = Regex {
            regex: RustRegex::new("").unwrap(),
            target: AccessorBuilder::new().build("", "").unwrap(),
        };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_operator_with_expected_arguments() {
        let operator =
            Regex::build("one", AccessorBuilder::new().build("", "two").unwrap()).unwrap();

        let event = Event::new("test_type");

        assert_eq!("one", operator.regex.to_string());
        assert_eq!(
            "two",
            operator.target.get(&(&json!(event), &mut Value::Null).into()).unwrap().as_ref()
        );
    }

    #[test]
    fn build_should_fail_if_invalid_regex() {
        let operator = Regex::build("[", AccessorBuilder::new().build("", "two").unwrap());
        assert!(operator.is_err());
    }

    #[test]
    fn should_evaluate_to_true_if_it_matches_the_regex() {
        let operator =
            Regex::build("[a-fA-F0-9]", AccessorBuilder::new().build("", "f").unwrap()).unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = Regex::build(
            "[a-fA-F0-9]",
            AccessorBuilder::new().build("", "${event.payload.name1}").unwrap(),
        )
        .unwrap();

        let mut payload = Map::new();
        payload.insert("name1".to_owned(), Value::String("F".to_owned()));
        payload.insert("name2".to_owned(), Value::String("G".to_owned()));

        let event = Event::new_with_payload("test_type", payload);

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_it_does_not_match_the_regex() {
        let operator = Regex::build(
            "[a-fA-F0-9]",
            AccessorBuilder::new().build("", "${event.payload.name2}").unwrap(),
        )
        .unwrap();

        let mut payload = Map::new();
        payload.insert("name1".to_owned(), Value::String("F".to_owned()));
        payload.insert("name2".to_owned(), Value::String("G".to_owned()));

        let event = Event::new_with_payload("test_type", payload);

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_field_does_not_exists() {
        let operator = Regex::build(
            "[^.{0}$]",
            AccessorBuilder::new().build("", "${event.payload.name}").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_value_of_type_bool() {
        let operator = Regex::build(
            "[a-fA-F0-9]",
            AccessorBuilder::new().build("", "${event.payload.value}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("value".to_owned(), Value::Bool(true));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_value_of_type_number() {
        let operator = Regex::build(
            "[a-fA-F0-9]",
            AccessorBuilder::new().build("", "${event.payload.value}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("value".to_owned(), json!(999));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }
}
