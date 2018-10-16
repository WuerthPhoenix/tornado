use config;
use error::MatcherError;
use operator::{Operator, OperatorBuilder};
use tornado_common::Event;

const OPERATOR_NAME: &str = "or";

/// A matching operator that evaluates whether at list one children on a list of rules matches.
#[derive(Debug)]
pub struct Or {
    rules: Vec<Box<Operator>>,
}

impl Or {
    pub fn build(
        args: &Vec<config::Operator>,
        builder: &OperatorBuilder,
    ) -> Result<Or, MatcherError> {
        let mut rules = vec![];
        for entry in args {
            let rule = builder.build(&entry)?;
            rules.push(rule)
        }
        Ok(Or { rules })
    }
}

impl Operator for Or {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &Event) -> bool {
        for rule in &self.rules {
            if rule.evaluate(event) {
                return true;
            }
        }
        return false;
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn should_return_the_rule_name() {
        let rule = Or { rules: vec![] };
        assert_eq!(OPERATOR_NAME, rule.name());
    }

    #[test]
    fn should_build_the_or_with_expected_arguments() {
        let rule = Or::build(
            &vec![config::Operator::Equals {
                first: "first_arg=".to_owned(),
                second: "second_arg".to_owned(),
            }],
            &OperatorBuilder::new(),
        ).unwrap();

        assert_eq!(1, rule.rules.len());
        assert_eq!("equal", rule.rules[0].name());
    }

    #[test]
    fn should_build_the_or_with_no_arguments() {
        let rule = Or::build(&vec![], &OperatorBuilder::new()).unwrap();
        assert_eq!(0, rule.rules.len());
    }

    #[test]
    fn build_should_fail_if_wrong_nested_rule() {
        let rule = Or::build(
            &vec![config::Operator::Equals {
                first: "${NOT_EXISTING}".to_owned(),
                second: "second_arg".to_owned(),
            }],
            &OperatorBuilder::new(),
        );
        assert!(rule.is_err());
    }
    #[test]
    fn build_should_be_recursive() {
        let rule = Or::build(
            &vec![
                config::Operator::Equals {
                    first: "1".to_owned(),
                    second: "2".to_owned(),
                },
                config::Operator::And {
                    operators: vec![config::Operator::Equals {
                        first: "3".to_owned(),
                        second: "4".to_owned(),
                    }],
                },
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        assert_eq!("or", rule.name());
        assert_eq!(2, rule.rules.len());
        assert_eq!("equal", rule.rules[0].name());
        assert_eq!("and", rule.rules[1].name());

        println!("{:?}", rule.rules[1]);

        assert!(
            format!("{:?}", rule.rules[1])
                .contains(r#"Equal { first_arg: ConstantAccessor { value: "3" }, second_arg: ConstantAccessor { value: "4" } }"#)
        )
    }

    #[test]
    fn should_evaluate_to_false_if_no_children() {
        let rule = Or::build(&vec![], &OperatorBuilder::new()).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_true_if_all_children_match() {
        let rule = Or::build(
            &vec![
                config::Operator::Equals {
                    first: "1".to_owned(),
                    second: "1".to_owned(),
                },
                config::Operator::Equals {
                    first: "2".to_owned(),
                    second: "2".to_owned(),
                },
                config::Operator::Equals {
                    first: "3".to_owned(),
                    second: "3".to_owned(),
                },
                config::Operator::Equals {
                    first: "4".to_owned(),
                    second: "4".to_owned(),
                },
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_true_if_at_least_a_children_matches() {
        let rule = Or::build(
            &vec![
                config::Operator::Equals {
                    first: "1".to_owned(),
                    second: "4".to_owned(),
                },
                config::Operator::Equals {
                    first: "2".to_owned(),
                    second: "4".to_owned(),
                },
                config::Operator::Equals {
                    first: "3".to_owned(),
                    second: "4".to_owned(),
                },
                config::Operator::Equals {
                    first: "4".to_owned(),
                    second: "4".to_owned(),
                },
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_false_if_no_children_match() {
        let rule = Or::build(
            &vec![
                config::Operator::Equals {
                    first: "1".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Equals {
                    first: "2".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Equals {
                    first: "3".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Equals {
                    first: "4".to_owned(),
                    second: "5".to_owned(),
                },
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_true_if_at_least_a_children_matches_recursively() {
        let rule = Or::build(
            &vec![
                config::Operator::Equals {
                    first: "1".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Equals {
                    first: "2".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Equals {
                    first: "3".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Or {
                    operators: vec![
                        config::Operator::Equals {
                            first: "4".to_owned(),
                            second: "5".to_owned(),
                        },
                        config::Operator::Equals {
                            first: "5".to_owned(),
                            second: "5".to_owned(),
                        },
                    ],
                },
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_false_if_no_children_match_recursively() {
        let rule = Or::build(
            &vec![
                config::Operator::Equals {
                    first: "1".to_owned(),
                    second: "6".to_owned(),
                },
                config::Operator::Equals {
                    first: "2".to_owned(),
                    second: "6".to_owned(),
                },
                config::Operator::Equals {
                    first: "3".to_owned(),
                    second: "6".to_owned(),
                },
                config::Operator::Or {
                    operators: vec![
                        config::Operator::Equals {
                            first: "4".to_owned(),
                            second: "6".to_owned(),
                        },
                        config::Operator::Equals {
                            first: "5".to_owned(),
                            second: "6".to_owned(),
                        },
                    ],
                },
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively() {
        let rule = Or::build(
            &vec![
                config::Operator::Equals {
                    first: "1".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Equals {
                    first: "2".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Equals {
                    first: "3".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Or {
                    operators: vec![
                        config::Operator::Equals {
                            first: "4".to_owned(),
                            second: "5".to_owned(),
                        },
                        config::Operator::Equals {
                            first: "type".to_owned(),
                            second: "${event.type}".to_owned(),
                        },
                    ],
                },
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "type".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively_and_return_false() {
        let rule = Or::build(
            &vec![
                config::Operator::Equals {
                    first: "1".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Equals {
                    first: "2".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Equals {
                    first: "3".to_owned(),
                    second: "5".to_owned(),
                },
                config::Operator::Or {
                    operators: vec![
                        config::Operator::Equals {
                            first: "4".to_owned(),
                            second: "5".to_owned(),
                        },
                        config::Operator::Equals {
                            first: "type1".to_owned(),
                            second: "${event.type}".to_owned(),
                        },
                    ],
                },
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "type".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

}
