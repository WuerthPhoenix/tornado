use accessor::{Accessor, AccessorBuilder};
use error::MatcherError;
use operator::Operator;
use tornado_common::Event;

const OPERATOR_NAME: &str = "equal";

/// A matching operator that evaluates whether two strings are equals.
#[derive(Debug)]
pub struct Equal {
    first_arg: Box<Accessor>,
    second_arg: Box<Accessor>,
}

impl Equal {
    pub fn build(
        args: &Vec<String>,
        accessor_builder: &AccessorBuilder,
    ) -> Result<Equal, MatcherError> {
        let expected = 2;
        if args.len() != expected {
            return Err(MatcherError::WrongNumberOfArgumentsError {
                operator: OPERATOR_NAME,
                expected: expected as u64,
                found: args.len() as u64,
            });
        }
        Ok(Equal {
            first_arg: accessor_builder.build(&args[0])?,
            second_arg: accessor_builder.build(&args[1])?,
        })
    }
}

impl Operator for Equal {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &Event) -> bool {
        self.first_arg.get(event) == self.second_arg.get(event)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn should_return_the_rule_name() {
        let rule = Equal {
            first_arg: AccessorBuilder::new().build(&"".to_owned()).unwrap(),
            second_arg: AccessorBuilder::new().build(&"".to_owned()).unwrap(),
        };
        assert_eq!(OPERATOR_NAME, rule.name());
    }

    #[test]
    fn should_build_the_rule_with_expected_arguments() {
        let rule = Equal::build(
            &vec!["one".to_string(), "two".to_string()],
            &AccessorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert_eq!("one".to_string(), rule.first_arg.get(&event).unwrap());
        assert_eq!("two".to_string(), rule.second_arg.get(&event).unwrap());
    }

    #[test]
    fn build_should_fail_if_not_enough_arguments() {
        let rule = Equal::build(&vec!["one".to_string()], &AccessorBuilder::new());
        assert!(rule.is_err());
    }

    #[test]
    fn build_should_fail_if_too_much_arguments() {
        let rule = Equal::build(
            &vec!["one".to_string(), "two".to_string(), "three".to_string()],
            &AccessorBuilder::new(),
        );
        assert!(rule.is_err());
    }

    #[test]
    fn should_evaluate_to_true_if_equal_arguments() {
        let rule = Equal::build(
            &vec!["one".to_string(), "one".to_string()],
            &AccessorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let rule = Equal::build(
            &vec!["${event.type}".to_string(), "test_type".to_string()],
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
    fn should_evaluate_to_false_if_different_arguments() {
        let rule = Equal::build(
            &vec!["${event.type}".to_string(), "wrong_test_type".to_string()],
            &AccessorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

    #[test]
    fn should_compare_event_fields() {
        let rule = Equal::build(
            &vec![
                "${event.type}".to_string(),
                "${event.payload.type}".to_string(),
            ],
            &AccessorBuilder::new(),
        ).unwrap();

        let mut payload = HashMap::new();
        payload.insert("type".to_owned(), "type".to_owned());

        let event = Event {
            payload,
            event_type: "type".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_return_true_if_fields_do_not_exist() {
        let rule = Equal::build(
            &vec![
                "${event.payload.2}".to_string(),
                "${event.payload.1}".to_string(),
            ],
            &AccessorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "type".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

}
