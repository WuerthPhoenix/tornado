use config;
use error::MatcherError;
use operator::{Operator, OperatorBuilder};
use tornado_common::Event;

const OPERATOR_NAME: &str = "and";

/// A matching operator that evaluates whether a list of children rules are all verified.
#[derive(Debug)]
pub struct And {
    operators: Vec<Box<Operator>>,
}

impl And {
    pub fn build(
        args: &[config::Operator],
        builder: &OperatorBuilder,
    ) -> Result<And, MatcherError> {
        let mut operators = vec![];
        for entry in args {
            let rule = builder.build(&entry)?;
            operators.push(rule)
        }
        Ok(And { operators })
    }
}

impl Operator for And {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &Event) -> bool {
        for rule in &self.operators {
            if !rule.evaluate(event) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn should_return_the_rule_name() {
        let rule = And { operators: vec![] };
        assert_eq!(OPERATOR_NAME, rule.name());
    }

    #[test]
    fn should_build_the_and_with_expected_arguments() {
        let rule = And::build(
            &vec![config::Operator::Equals {
                first: "first_arg=".to_owned(),
                second: "second_arg".to_owned(),
            }],
            &OperatorBuilder::new(),
        ).unwrap();
        assert_eq!(1, rule.operators.len());
        assert_eq!("equal", rule.operators[0].name());
    }

    #[test]
    fn should_build_the_and_with_no_arguments() {
        let rule = And::build(&vec![], &OperatorBuilder::new()).unwrap();
        assert_eq!(0, rule.operators.len());
    }

    #[test]
    fn build_should_fail_if_wrong_nested_rule() {
        let rule = And::build(
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
        let rule = And::build(
            &vec![
                config::Operator::Equals {
                    first: "1".to_owned(),
                    second: "2".to_owned(),
                },
                config::Operator::Or {
                    operators: vec![config::Operator::Equals {
                        first: "3".to_owned(),
                        second: "4".to_owned(),
                    }],
                },
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        assert_eq!("and", rule.name());
        assert_eq!(2, rule.operators.len());
        assert_eq!("equal", rule.operators[0].name());
        assert_eq!("or", rule.operators[1].name());

        println!("{:?}", rule.operators[1]);

        assert!(format!("{:?}", rule.operators[1]).contains(
            r#"Equal { first_arg: Constant { value: "3" }, second_arg: Constant { value: "4" } }"#
        ))
    }

    #[test]
    fn should_evaluate_to_true_if_no_children() {
        let rule = And::build(&vec![], &OperatorBuilder::new()).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_true_if_all_children_match() {
        let rule = And::build(
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
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_false_if_not_all_children_match() {
        let rule = And::build(
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
                    second: "1".to_owned(),
                },
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_true_if_all_children_match_recursively() {
        let rule = And::build(
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
                config::Operator::And {
                    operators: vec![
                        config::Operator::Equals {
                            first: "4".to_owned(),
                            second: "4".to_owned(),
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
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_false_if_not_all_children_match_recursively() {
        let rule = And::build(
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
                config::Operator::And {
                    operators: vec![
                        config::Operator::Equals {
                            first: "4".to_owned(),
                            second: "4".to_owned(),
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
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively() {
        let rule = And::build(
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
                config::Operator::And {
                    operators: vec![
                        config::Operator::Equals {
                            first: "4".to_owned(),
                            second: "4".to_owned(),
                        },
                        config::Operator::Equals {
                            first: "${event.type}".to_owned(),
                            second: "type".to_owned(),
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
        let rule = And::build(
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
                config::Operator::And {
                    operators: vec![
                        config::Operator::Equals {
                            first: "4".to_owned(),
                            second: "4".to_owned(),
                        },
                        config::Operator::Equals {
                            first: "${event.type}".to_owned(),
                            second: "type1".to_owned(),
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
