use accessor::{Accessor, AccessorBuilder};
use error::MatcherError;
use operator::Operator;
use regex::Regex as RustRegex;
use tornado_common::Event;

const OPERATOR_NAME: &str = "regex";

/// A matching operator that evaluates whether a string matches a regex.
#[derive(Debug)]
pub struct Regex {
    regex: RustRegex,
    target: Box<Accessor>,
}

impl Regex {
    pub fn build(
        args: &Vec<String>,
        accessor_builder: &AccessorBuilder,
    ) -> Result<Regex, MatcherError> {
        let expected = 2;
        if args.len() != expected {
            return Err(MatcherError::WrongNumberOfArgumentsError {
                operator: OPERATOR_NAME,
                expected: expected as u64,
                found: args.len() as u64,
            });
        }
        let regex_string = args[0].clone();
        let regex_str = regex_string.as_str();
        let target = accessor_builder.build(&args[1])?;
        let regex =
            RustRegex::new(regex_str).map_err(|e| MatcherError::OperatorBuildFailError {
                message: format!("Cannot parse regex [{}]", regex_str),
                cause: e.to_string(),
            })?;

        Ok(Regex { target, regex })
    }
}

impl Operator for Regex {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &Event) -> bool {
        self.target
            .get(event)
            .map_or(false, |value| self.regex.is_match(value.as_str()))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn should_return_the_rule_name() {
        let rule = Regex {
            regex: RustRegex::new("").unwrap(),
            target: AccessorBuilder::new().build(&"".to_owned()).unwrap(),
        };
        assert_eq!(OPERATOR_NAME, rule.name());
    }

    #[test]
    fn should_build_the_rule_with_expected_arguments() {
        let rule = Regex::build(
            &vec!["one".to_string(), "two".to_string()],
            &AccessorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert_eq!("one".to_string(), rule.regex.to_string());
        assert_eq!("two".to_string(), rule.target.get(&event).unwrap());
    }

    #[test]
    fn build_should_fail_if_not_enough_arguments() {
        let rule = Regex::build(&vec!["one".to_string()], &AccessorBuilder::new());
        assert!(rule.is_err());
    }

    #[test]
    fn build_should_fail_if_too_much_arguments() {
        let rule = Regex::build(
            &vec!["one".to_string(), "two".to_string(), "three".to_string()],
            &AccessorBuilder::new(),
        );
        assert!(rule.is_err());
    }

    #[test]
    fn build_should_fail_if_invalid_regex() {
        let rule = Regex::build(
            &vec!["[".to_string(), "two".to_string()],
            &AccessorBuilder::new(),
        );
        assert!(rule.is_err());
    }

    #[test]
    fn should_evaluate_to_true_if_it_matches_the_regex() {
        let rule = Regex::build(
            &vec!["[a-fA-F0-9]".to_string(), "f".to_string()],
            &AccessorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let rule = Regex::build(
            &vec![
                "[a-fA-F0-9]".to_string(),
                "${event.payload.name1}".to_string(),
            ],
            &AccessorBuilder::new(),
        ).unwrap();

        let mut payload = HashMap::new();
        payload.insert("name1".to_owned(), "F".to_owned());
        payload.insert("name2".to_owned(), "G".to_owned());

        let event = Event {
            payload,
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_false_if_it_does_not_match_the_regex() {
        let rule = Regex::build(
            &vec![
                "[a-fA-F0-9]".to_string(),
                "${event.payload.name2}".to_string(),
            ],
            &AccessorBuilder::new(),
        ).unwrap();

        let mut payload = HashMap::new();
        payload.insert("name1".to_owned(), "F".to_owned());
        payload.insert("name2".to_owned(), "G".to_owned());

        let event = Event {
            payload,
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_false_if_field_does_not_exists() {
        let rule = Regex::build(
            &vec!["[^.{0}$]".to_string(), "${event.payload.name}".to_string()],
            &AccessorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }
}
