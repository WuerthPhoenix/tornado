use accessor::Accessor;
use error::MatcherError;
use operator::Operator;
use regex::Regex as RustRegex;
use tornado_common::Event;

const OPERATOR_NAME: &str = "regex";

/// A matching operator that evaluates whether a string matches a regex.
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

    fn evaluate(&self, event: &Event) -> bool {
        self.target
            .get(event)
            .map_or(false, |value| self.regex.is_match(value.as_str()))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use accessor::AccessorBuilder;
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
            &"one".to_owned(),
            AccessorBuilder::new().build(&"two".to_owned()).unwrap(),
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
    fn build_should_fail_if_invalid_regex() {
        let rule = Regex::build(
            &"[".to_owned(),
            AccessorBuilder::new().build(&"two".to_owned()).unwrap(),
        );
        assert!(rule.is_err());
    }

    #[test]
    fn should_evaluate_to_true_if_it_matches_the_regex() {
        let rule = Regex::build(
            &"[a-fA-F0-9]".to_owned(),
            AccessorBuilder::new().build(&"f".to_owned()).unwrap(),
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
            &"[a-fA-F0-9]".to_owned(),
            AccessorBuilder::new()
                .build(&"${event.payload.name1}".to_owned())
                .unwrap(),
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
            &"[a-fA-F0-9]".to_owned(),
            AccessorBuilder::new()
                .build(&"${event.payload.name2}".to_owned())
                .unwrap(),
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
            &"[^.{0}$]".to_owned(),
            AccessorBuilder::new()
                .build(&"${event.payload.name}".to_owned())
                .unwrap(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

}
