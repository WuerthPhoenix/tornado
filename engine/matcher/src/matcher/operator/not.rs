use crate::config;
use crate::error::MatcherError;
use crate::matcher::operator::{Operator, OperatorBuilder};
use crate::model::InternalEvent;

const OPERATOR_NAME: &str = "not";

/// A matching matcher.operator that negates the evaluation of the child operator
#[derive(Debug)]
pub struct Not {
    operator: Box<dyn Operator>,
}

impl Not {
    pub fn build(
        rule_name: &str,
        args: &config::rule::Operator,
        builder: &OperatorBuilder,
    ) -> Result<Not, MatcherError> {
        let operator = builder.build(rule_name, args)?;
        Ok(Not { operator })
    }
}

impl Operator for Not {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &InternalEvent) -> bool {
        !self.operator.evaluate(event)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::matcher::operator::Operator;
    use serde_json::json;
    use tornado_common_api::{Event, Value};

    #[test]
    fn should_return_the_operator_name() {
        let operator = Not::build(
            "",
            &config::rule::Operator::Equals {
                first: Value::String("first_arg=".to_owned()),
                second: Value::String("second_arg".to_owned()),
            },
            &OperatorBuilder::new(),
        )
        .unwrap();
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_not_with_expected_arguments() {
        let operator = Not::build(
            "",
            &config::rule::Operator::Equals {
                first: Value::String("first_arg=".to_owned()),
                second: Value::String("second_arg".to_owned()),
            },
            &OperatorBuilder::new(),
        )
        .unwrap();
        assert_eq!("equals", operator.operator.name());
    }

    #[test]
    fn build_should_fail_if_wrong_nested_operator() {
        let operator = Not::build(
            "",
            &config::rule::Operator::Equals {
                first: Value::String("${NOT_EXISTING}".to_owned()),
                second: Value::String("second_arg".to_owned()),
            },
            &OperatorBuilder::new(),
        );
        assert!(operator.is_err());
    }

    #[test]
    fn build_should_be_recursive() {
        let operator = Not::build(
            "",
            &config::rule::Operator::Equals {
                first: Value::String("1".to_owned()),
                second: Value::String("2".to_owned()),
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        assert_eq!("not", operator.name());
        assert_eq!("equals", operator.operator.name());
    }

    #[test]
    fn should_evaluate_to_true_if_child_is_fales() {
        let operator = Not::build(
            "",
            &config::rule::Operator::Equals {
                first: Value::String("1".to_owned()),
                second: Value::String("2".to_owned()),
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &Value::Null, &Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_child_is_true() {
        let operator = Not::build(
            "",
            &config::rule::Operator::Equals {
                first: Value::String("1".to_owned()),
                second: Value::String("1".to_owned()),
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("");

        assert!(!operator.evaluate(&(&json!(event), &Value::Null, &Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_double_negation_of_true() {
        let operator = Not::build(
            "",
            &config::rule::Operator::Not {
                operator: Box::new(config::rule::Operator::Equals {
                    first: Value::String("4".to_owned()),
                    second: Value::String("4".to_owned()),
                }),
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("");

        assert!(operator.evaluate(&(&json!(event), &Value::Null, &Value::Null).into()));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively() {
        let operator = Not::build(
            "",
            &config::rule::Operator::And {
                operators: vec![
                    config::rule::Operator::Equals {
                        first: Value::String("4".to_owned()),
                        second: Value::String("4".to_owned()),
                    },
                    config::rule::Operator::Equals {
                        first: Value::String("${event.type}".to_owned()),
                        second: Value::String("type".to_owned()),
                    },
                ],
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("type");

        assert!(!operator.evaluate(&(&json!(event), &Value::Null, &Value::Null).into()));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively_and_return_true() {
        let operator = Not::build(
            "",
            &config::rule::Operator::And {
                operators: vec![
                    config::rule::Operator::Equals {
                        first: Value::String("4".to_owned()),
                        second: Value::String("4".to_owned()),
                    },
                    config::rule::Operator::Equals {
                        first: Value::String("${event.type}".to_owned()),
                        second: Value::String("type1".to_owned()),
                    },
                ],
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("type");

        assert!(operator.evaluate(&(&json!(event), &Value::Null, &Value::Null).into()));
    }
}
