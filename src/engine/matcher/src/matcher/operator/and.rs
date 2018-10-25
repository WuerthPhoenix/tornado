use config;
use error::MatcherError;
use matcher::operator::{Operator, OperatorBuilder};
use model::ProcessedEvent;

const OPERATOR_NAME: &str = "and";

/// A matching matcher.operator that evaluates whether a list of children operators are all verified.
#[derive(Debug)]
pub struct And {
    operators: Vec<Box<Operator>>,
}

impl And {
    pub fn build(
        rule_name: &str,
        args: &[config::Operator],
        builder: &OperatorBuilder,
    ) -> Result<And, MatcherError> {
        let mut operators = vec![];
        for entry in args {
            let operator = builder.build(rule_name, &entry)?;
            operators.push(operator)
        }
        Ok(And { operators })
    }
}

impl Operator for And {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &ProcessedEvent) -> bool {
        for operator in &self.operators {
            if !operator.evaluate(event) {
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
    use tornado_common_api::Event;

    #[test]
    fn should_return_the_operator_name() {
        let operator = And { operators: vec![] };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_and_with_expected_arguments() {
        let operator = And::build(
            "",
            &vec![config::Operator::Equal {
                first: "first_arg=".to_owned(),
                second: "second_arg".to_owned(),
            }],
            &OperatorBuilder::new(),
        ).unwrap();
        assert_eq!(1, operator.operators.len());
        assert_eq!("equal", operator.operators[0].name());
    }

    #[test]
    fn should_build_the_and_with_no_arguments() {
        let operator = And::build("", &vec![], &OperatorBuilder::new()).unwrap();
        assert_eq!(0, operator.operators.len());
    }

    #[test]
    fn build_should_fail_if_wrong_nested_operator() {
        let operator = And::build(
            "",
            &vec![config::Operator::Equal {
                first: "${NOT_EXISTING}".to_owned(),
                second: "second_arg".to_owned(),
            }],
            &OperatorBuilder::new(),
        );
        assert!(operator.is_err());
    }

    #[test]
    fn build_should_be_recursive() {
        let operator = And::build(
            "",
            &vec![
                config::Operator::Equal {
                    first: "1".to_owned(),
                    second: "2".to_owned(),
                },
                config::Operator::Or {
                    operators: vec![config::Operator::Equal {
                        first: "3".to_owned(),
                        second: "4".to_owned(),
                    }],
                },
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        assert_eq!("and", operator.name());
        assert_eq!(2, operator.operators.len());
        assert_eq!("equal", operator.operators[0].name());
        assert_eq!("or", operator.operators[1].name());

        println!("{:?}", operator.operators[1]);

        assert!(format!("{:?}", operator.operators[1]).contains(
            r#"Equal { first_arg: Constant { value: "3" }, second_arg: Constant { value: "4" } }"#
        ))
    }

    #[test]
    fn should_evaluate_to_true_if_no_children() {
        let operator = And::build("", &vec![], &OperatorBuilder::new()).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_evaluate_to_true_if_all_children_match() {
        let operator = And::build(
            "",
            &vec![
                config::Operator::Equal {
                    first: "1".to_owned(),
                    second: "1".to_owned(),
                },
                config::Operator::Equal {
                    first: "2".to_owned(),
                    second: "2".to_owned(),
                },
                config::Operator::Equal {
                    first: "3".to_owned(),
                    second: "3".to_owned(),
                },
                config::Operator::Equal {
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

        assert!(operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_evaluate_to_false_if_not_all_children_match() {
        let operator = And::build(
            "",
            &vec![
                config::Operator::Equal {
                    first: "1".to_owned(),
                    second: "1".to_owned(),
                },
                config::Operator::Equal {
                    first: "2".to_owned(),
                    second: "2".to_owned(),
                },
                config::Operator::Equal {
                    first: "3".to_owned(),
                    second: "3".to_owned(),
                },
                config::Operator::Equal {
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

        assert!(!operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_evaluate_to_true_if_all_children_match_recursively() {
        let operator = And::build(
            "",
            &vec![
                config::Operator::Equal {
                    first: "1".to_owned(),
                    second: "1".to_owned(),
                },
                config::Operator::Equal {
                    first: "2".to_owned(),
                    second: "2".to_owned(),
                },
                config::Operator::Equal {
                    first: "3".to_owned(),
                    second: "3".to_owned(),
                },
                config::Operator::And {
                    operators: vec![
                        config::Operator::Equal {
                            first: "4".to_owned(),
                            second: "4".to_owned(),
                        },
                        config::Operator::Equal {
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

        assert!(operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_evaluate_to_false_if_not_all_children_match_recursively() {
        let operator = And::build(
            "",
            &vec![
                config::Operator::Equal {
                    first: "1".to_owned(),
                    second: "1".to_owned(),
                },
                config::Operator::Equal {
                    first: "2".to_owned(),
                    second: "2".to_owned(),
                },
                config::Operator::Equal {
                    first: "3".to_owned(),
                    second: "3".to_owned(),
                },
                config::Operator::And {
                    operators: vec![
                        config::Operator::Equal {
                            first: "4".to_owned(),
                            second: "4".to_owned(),
                        },
                        config::Operator::Equal {
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

        assert!(!operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively() {
        let operator = And::build(
            "",
            &vec![
                config::Operator::Equal {
                    first: "1".to_owned(),
                    second: "1".to_owned(),
                },
                config::Operator::Equal {
                    first: "2".to_owned(),
                    second: "2".to_owned(),
                },
                config::Operator::Equal {
                    first: "3".to_owned(),
                    second: "3".to_owned(),
                },
                config::Operator::And {
                    operators: vec![
                        config::Operator::Equal {
                            first: "4".to_owned(),
                            second: "4".to_owned(),
                        },
                        config::Operator::Equal {
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

        assert!(operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively_and_return_false() {
        let operator = And::build(
            "",
            &vec![
                config::Operator::Equal {
                    first: "1".to_owned(),
                    second: "1".to_owned(),
                },
                config::Operator::Equal {
                    first: "2".to_owned(),
                    second: "2".to_owned(),
                },
                config::Operator::Equal {
                    first: "3".to_owned(),
                    second: "3".to_owned(),
                },
                config::Operator::And {
                    operators: vec![
                        config::Operator::Equal {
                            first: "4".to_owned(),
                            second: "4".to_owned(),
                        },
                        config::Operator::Equal {
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

        assert!(!operator.evaluate(&ProcessedEvent::new(event)));
    }

}
