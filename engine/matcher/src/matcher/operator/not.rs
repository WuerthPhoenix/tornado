use crate::config;
use crate::error::MatcherError;
use crate::matcher::operator::{Operator, OperatorBuilder};
use crate::model::InternalEvent;
use tornado_common_api::Value;

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
        let operator = builder.build(rule_name, &args)?;
        Ok(Not { operator })
    }
}

impl Operator for Not {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &InternalEvent, extracted_vars: Option<&Value>) -> bool {
        !self.operator.evaluate(event, extracted_vars)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::matcher::operator::Operator;
    use tornado_common_api::Event;

    #[test]
    fn should_return_the_operator_name() {
        let operator = Not::build(
            "",
            &config::rule::Operator::Equals {
                first: Value::Text("first_arg=".to_owned()),
                second: Value::Text("second_arg".to_owned()),
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
                first: Value::Text("first_arg=".to_owned()),
                second: Value::Text("second_arg".to_owned()),
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
                first: Value::Text("${NOT_EXISTING}".to_owned()),
                second: Value::Text("second_arg".to_owned()),
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
                first: Value::Text("1".to_owned()),
                second: Value::Text("2".to_owned()),
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        assert_eq!("not", operator.name());
        assert_eq!("equals", operator.operator.name());

        println!("{:?}", operator.operator);

        assert!(format!("{:?}", operator.operator).contains(
            r#"Equals { first_arg: Constant { value: Text("1") }, second_arg: Constant { value: Text("2") } }"#
        ))
    }

    #[test]
    fn should_evaluate_to_true_if_child_is_fales() {
        let operator = Not::build(
            "",
            &config::rule::Operator::Equals {
                first: Value::Text("1".to_owned()),
                second: Value::Text("2".to_owned()),
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert_eq!(operator.evaluate(&InternalEvent::new(event), None), true);
    }

    #[test]
    fn should_evaluate_to_false_if_child_is_true() {
        let operator = Not::build(
            "",
            &config::rule::Operator::Equals {
                first: Value::Text("1".to_owned()),
                second: Value::Text("1".to_owned()),
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("");

        assert_eq!(operator.evaluate(&InternalEvent::new(event), None), false);
    }

    #[test]
    fn should_evaluate_to_true_if_double_negation_of_true() {
        let operator = Not::build(
            "",
            &config::rule::Operator::Not {
                operator: Box::new(config::rule::Operator::Equals {
                    first: Value::Text("4".to_owned()),
                    second: Value::Text("4".to_owned()),
                }),
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("");

        assert_eq!(operator.evaluate(&InternalEvent::new(event), None), true);
    }

    #[test]
    fn should_evaluate_using_accessors_recursively() {
        let operator = Not::build(
            "",
            &config::rule::Operator::And {
                operators: vec![
                    config::rule::Operator::Equals {
                        first: Value::Text("4".to_owned()),
                        second: Value::Text("4".to_owned()),
                    },
                    config::rule::Operator::Equals {
                        first: Value::Text("${event.type}".to_owned()),
                        second: Value::Text("type".to_owned()),
                    },
                ],
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively_and_return_true() {
        let operator = Not::build(
            "",
            &config::rule::Operator::And {
                operators: vec![
                    config::rule::Operator::Equals {
                        first: Value::Text("4".to_owned()),
                        second: Value::Text("4".to_owned()),
                    },
                    config::rule::Operator::Equals {
                        first: Value::Text("${event.type}".to_owned()),
                        second: Value::Text("type1".to_owned()),
                    },
                ],
            },
            &OperatorBuilder::new(),
        )
        .unwrap();

        let event = Event::new("type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }
}
