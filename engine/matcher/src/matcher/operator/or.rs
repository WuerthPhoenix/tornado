use crate::config;
use crate::error::MatcherError;
use crate::matcher::operator::{Operator, OperatorBuilder};
use crate::model::InternalEvent;
use tornado_common_api::Value;

const OPERATOR_NAME: &str = "or";

/// A matching matcher.operator that checks whether at least one child on a list of operators has been verified
#[derive(Debug)]
pub struct Or {
    operators: Vec<Box<dyn Operator>>,
}

impl Or {
    pub fn build(
        rule_name: &str,
        args: &[config::rule::Operator],
        builder: &OperatorBuilder,
    ) -> Result<Or, MatcherError> {
        let mut operators = vec![];
        for entry in args {
            let operator = builder.build(rule_name, &entry)?;
            operators.push(operator)
        }
        Ok(Or { operators })
    }
}

impl Operator for Or {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &InternalEvent, extracted_vars: Option<&Value>) -> bool {
        self.operators.iter().any(|op| op.evaluate(event, extracted_vars))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use tornado_common_api::Event;

    #[test]
    fn should_return_the_operator_name() {
        let operator = Or { operators: vec![] };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_or_with_expected_arguments() {
        let operator = Or::build(
            "",
            &vec![config::rule::Operator::Equals {
                first: Value::Text("first_arg=".to_owned()),
                second: Value::Text("second_arg".to_owned()),
            }],
            &OperatorBuilder::new(),
        )
        .unwrap();

        assert_eq!(1, operator.operators.len());
        assert_eq!("equals", operator.operators[0].name());
    }

    #[test]
    fn should_build_the_or_with_no_arguments() {
        let operator = Or::build("", &vec![], &OperatorBuilder::new()).unwrap();
        assert_eq!(0, operator.operators.len());
    }

    #[test]
    fn build_should_fail_if_wrong_nested_operator() {
        let operator = Or::build(
            "",
            &vec![config::rule::Operator::Equals {
                first: Value::Text("${NOT_EXISTING}".to_owned()),
                second: Value::Text("second_arg".to_owned()),
            }],
            &OperatorBuilder::new(),
        );
        assert!(operator.is_err());
    }
    #[test]
    fn build_should_be_recursive() {
        let operator = Or::build(
            "",
            &vec![
                config::rule::Operator::Equals {
                    first: Value::Text("1".to_owned()),
                    second: Value::Text("2".to_owned()),
                },
                config::rule::Operator::And {
                    operators: vec![config::rule::Operator::Equals {
                        first: Value::Text("3".to_owned()),
                        second: Value::Text("4".to_owned()),
                    }],
                },
            ],
            &OperatorBuilder::new(),
        )
        .unwrap();

        assert_eq!("or", operator.name());
        assert_eq!(2, operator.operators.len());
        assert_eq!("equals", operator.operators[0].name());
        assert_eq!("and", operator.operators[1].name());

        println!("{:?}", operator.operators[1]);

        assert!(format!("{:?}", operator.operators[1]).contains(
            r#"Equals { first_arg: Constant { value: Text("3") }, second_arg: Constant { value: Text("4") } }"#
        ))
    }

    #[test]
    fn should_evaluate_to_false_if_no_children() {
        let operator = Or::build("", &vec![], &OperatorBuilder::new()).unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_all_children_match() {
        let operator = Or::build(
            "",
            &vec![
                config::rule::Operator::Equals {
                    first: Value::Text("1".to_owned()),
                    second: Value::Text("1".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("2".to_owned()),
                    second: Value::Text("2".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("3".to_owned()),
                    second: Value::Text("3".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("4".to_owned()),
                    second: Value::Text("4".to_owned()),
                },
            ],
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_at_least_a_children_matches() {
        let operator = Or::build(
            "",
            &vec![
                config::rule::Operator::Equals {
                    first: Value::Text("1".to_owned()),
                    second: Value::Text("4".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("2".to_owned()),
                    second: Value::Text("4".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("3".to_owned()),
                    second: Value::Text("4".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("4".to_owned()),
                    second: Value::Text("4".to_owned()),
                },
            ],
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_no_children_match() {
        let operator = Or::build(
            "",
            &vec![
                config::rule::Operator::Equals {
                    first: Value::Text("1".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("2".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("3".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("4".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
            ],
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_at_least_a_children_matches_recursively() {
        let operator = Or::build(
            "",
            &vec![
                config::rule::Operator::Equals {
                    first: Value::Text("1".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("2".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("3".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Or {
                    operators: vec![
                        config::rule::Operator::Equals {
                            first: Value::Text("4".to_owned()),
                            second: Value::Text("5".to_owned()),
                        },
                        config::rule::Operator::Equals {
                            first: Value::Text("5".to_owned()),
                            second: Value::Text("5".to_owned()),
                        },
                    ],
                },
            ],
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_no_children_match_recursively() {
        let operator = Or::build(
            "",
            &vec![
                config::rule::Operator::Equals {
                    first: Value::Text("1".to_owned()),
                    second: Value::Text("6".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("2".to_owned()),
                    second: Value::Text("6".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("3".to_owned()),
                    second: Value::Text("6".to_owned()),
                },
                config::rule::Operator::Or {
                    operators: vec![
                        config::rule::Operator::Equals {
                            first: Value::Text("4".to_owned()),
                            second: Value::Text("6".to_owned()),
                        },
                        config::rule::Operator::Equals {
                            first: Value::Text("5".to_owned()),
                            second: Value::Text("6".to_owned()),
                        },
                    ],
                },
            ],
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively() {
        let operator = Or::build(
            "",
            &vec![
                config::rule::Operator::Equals {
                    first: Value::Text("1".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("2".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("3".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Or {
                    operators: vec![
                        config::rule::Operator::Equals {
                            first: Value::Text("4".to_owned()),
                            second: Value::Text("5".to_owned()),
                        },
                        config::rule::Operator::Equals {
                            first: Value::Text("type".to_owned()),
                            second: Value::Text("${event.type}".to_owned()),
                        },
                    ],
                },
            ],
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively_and_return_false() {
        let operator = Or::build(
            "",
            &vec![
                config::rule::Operator::Equals {
                    first: Value::Text("1".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("2".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Equals {
                    first: Value::Text("3".to_owned()),
                    second: Value::Text("5".to_owned()),
                },
                config::rule::Operator::Or {
                    operators: vec![
                        config::rule::Operator::Equals {
                            first: Value::Text("4".to_owned()),
                            second: Value::Text("5".to_owned()),
                        },
                        config::rule::Operator::Equals {
                            first: Value::Text("type1".to_owned()),
                            second: Value::Text("${event.type}".to_owned()),
                        },
                    ],
                },
            ],
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }
}
