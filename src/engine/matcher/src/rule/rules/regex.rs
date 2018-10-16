use accessor::{Accessor, AccessorBuilder};
use error::MatcherError;
use regex::Regex;
use rule::Rule;
use tornado_common::Event;

const RULE_NAME: &str = "regex";

/// A matching rule that evaluates whether a string matches a regex.
#[derive(Debug)]
pub struct RegexRule {
    regex: Regex,
    target: Box<Accessor>,
}

impl RegexRule {
    pub fn build(
        args: &Vec<String>,
        accessor_builder: &AccessorBuilder,
    ) -> Result<RegexRule, MatcherError> {
        let expected = 2;
        if args.len() != expected {
            return Err(MatcherError::WrongNumberOfArgumentsError {
                rule: RULE_NAME,
                expected: expected as u64,
                found: args.len() as u64,
            });
        }
        let regex_string = args[0].clone();
        let regex_str = regex_string.as_str();
        let target = accessor_builder.build(&args[1])?;
        let regex = Regex::new(regex_str).map_err(|e| MatcherError::OperatorBuildFailError {
            message: format!("Cannot parse regex [{}]", regex_str),
            cause: e.to_string(),
        })?;

        Ok(RegexRule { target, regex })
    }
}

impl Rule for RegexRule {
    fn name(&self) -> &str {
        RULE_NAME
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
        let rule = RegexRule {
            regex: Regex::new("").unwrap(),
            target: AccessorBuilder::new().build(&"".to_owned()).unwrap(),
        };
        assert_eq!(RULE_NAME, rule.name());
    }

    #[test]
    fn should_build_the_rule_with_expected_arguments() {
        let rule = RegexRule::build(
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
        let rule = RegexRule::build(&vec!["one".to_string()], &AccessorBuilder::new());
        assert!(rule.is_err());
    }

    #[test]
    fn build_should_fail_if_too_much_arguments() {
        let rule = RegexRule::build(
            &vec!["one".to_string(), "two".to_string(), "three".to_string()],
            &AccessorBuilder::new(),
        );
        assert!(rule.is_err());
    }

    #[test]
    fn build_should_fail_if_invalid_regex() {
        let rule = RegexRule::build(
            &vec!["[".to_string(), "two".to_string()],
            &AccessorBuilder::new(),
        );
        assert!(rule.is_err());
    }

    #[test]
    fn should_evaluate_to_true_if_it_matches_the_regex() {
        let rule = RegexRule::build(
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
        let rule = RegexRule::build(
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
        let rule = RegexRule::build(
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
        let rule = RegexRule::build(
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
