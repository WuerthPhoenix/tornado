//! The operator module contains the logic to build a Rule's operators based on the
//! Rule configuration.
//!
//! An *Operator* is linked to the "WHERE" clause of a Rule and determines whether the rule
//! is matched by an Event.

use crate::accessor::AccessorBuilder;
use crate::config::rule;
use crate::error::MatcherError;
use crate::model::InternalEvent;
use log::*;
use std::fmt;
use tornado_common_api::Value;

pub mod and;
pub mod contain;
pub mod equal;
pub mod ge;
pub mod gt;
pub mod le;
pub mod lt;
pub mod or;
pub mod regex;
pub mod true_operator;

/// The Trait for a generic matcher.operator
pub trait Operator: fmt::Debug + Send + Sync {
    /// Returns the Operator name.
    fn name(&self) -> &str;

    /// Executes the current matcher.operator on a target Event and returns whether the Event matches it.
    fn evaluate(&self, event: &InternalEvent, extracted_vars: Option<&Value>) -> bool;
}

/// The Operator instance builder
#[derive(Default)]
pub struct OperatorBuilder {
    accessor: AccessorBuilder,
}

impl OperatorBuilder {
    pub fn new() -> OperatorBuilder {
        OperatorBuilder { accessor: AccessorBuilder::new() }
    }

    pub fn build_option(
        &self,
        rule_name: &str,
        config: &Option<rule::Operator>,
    ) -> Result<Box<dyn Operator>, MatcherError> {
        let result: Result<Box<dyn Operator>, MatcherError> = match config {
            Some(operator) => self.build(rule_name, operator),
            None => Ok(OperatorBuilder::default_operator()),
        };

        trace!(
            "OperatorBuilder - build: return matcher.operator [{:?}] for input value [{:?}]",
            &result,
            config
        );
        result
    }

    fn default_operator() -> Box<dyn Operator> {
        Box::new(crate::matcher::operator::true_operator::True {})
    }

    /// Returns a specific Operator instance based on the matcher.operator configuration.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// use tornado_engine_matcher::matcher::operator::OperatorBuilder;
    /// use tornado_engine_matcher::config::rule;
    /// use tornado_common_api::Value;
    ///
    /// let ops = rule::Operator::Equal {
    ///              first: Value::Text("${event.type}".to_owned()),
    ///              second: Value::Text("email".to_owned()),
    ///           };
    ///
    /// let builder = OperatorBuilder::new();
    /// let operator = builder.build("rule_name", &ops).unwrap(); // operator is an instance of Equal
    /// ```
    pub fn build(
        &self,
        rule_name: &str,
        config: &rule::Operator,
    ) -> Result<Box<dyn Operator>, MatcherError> {
        let result: Result<Box<dyn Operator>, MatcherError> = match config {
            rule::Operator::And { operators } => {
                Ok(Box::new(crate::matcher::operator::and::And::build("", &operators, self)?))
            }
            rule::Operator::Or { operators } => {
                Ok(Box::new(crate::matcher::operator::or::Or::build("", &operators, self)?))
            }
            rule::Operator::Equal { first, second } => {
                Ok(Box::new(crate::matcher::operator::equal::Equal::build(
                    self.accessor.build_from_value(rule_name, first)?,
                    self.accessor.build_from_value(rule_name, second)?,
                )?))
            }
            rule::Operator::GreaterEqualThan { first, second } => {
                Ok(Box::new(crate::matcher::operator::ge::GreaterEqualThan::build(
                    self.accessor.build_from_value(rule_name, first)?,
                    self.accessor.build_from_value(rule_name, second)?,
                )?))
            }
            rule::Operator::GreaterThan { first, second } => {
                Ok(Box::new(crate::matcher::operator::gt::GreaterThan::build(
                    self.accessor.build_from_value(rule_name, first)?,
                    self.accessor.build_from_value(rule_name, second)?,
                )?))
            }
            rule::Operator::LessEqualThan { first, second } => {
                Ok(Box::new(crate::matcher::operator::le::LessEqualThan::build(
                    self.accessor.build_from_value(rule_name, first)?,
                    self.accessor.build_from_value(rule_name, second)?,
                )?))
            }
            rule::Operator::LessThan { first, second } => {
                Ok(Box::new(crate::matcher::operator::lt::LessThan::build(
                    self.accessor.build_from_value(rule_name, first)?,
                    self.accessor.build_from_value(rule_name, second)?,
                )?))
            }
            rule::Operator::Contain { first, second } => {
                Ok(Box::new(crate::matcher::operator::contain::Contain::build(
                    self.accessor.build_from_value(rule_name, first)?,
                    self.accessor.build_from_value(rule_name, second)?,
                )?))
            }
            rule::Operator::Regex { regex, target } => {
                Ok(Box::new(crate::matcher::operator::regex::Regex::build(
                    regex,
                    self.accessor.build(rule_name, target)?,
                )?))
            }
            rule::Operator::None => Ok(OperatorBuilder::default_operator())
        };

        trace!(
            "OperatorBuilder - build: return matcher.operator [{:?}] for input value [{:?}]",
            &result,
            config
        );
        result
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn build_should_return_error_if_wrong_operator() {
        let ops = rule::Operator::Equal {
            first: Value::Text("${WRONG_ARG}".to_owned()),
            second: Value::Text("second_arg".to_owned()),
        };

        let builder = OperatorBuilder::new();
        assert!(builder.build_option("", &Some(ops)).is_err());
    }

    #[test]
    fn build_should_return_the_equal_operator() {
        let ops = rule::Operator::Equal {
            first: Value::Text("first_arg=".to_owned()),
            second: Value::Text("second_arg".to_owned()),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("equal", operator.name());
    }

    #[test]
    fn build_should_return_the_greater_equal_operator() {
        let ops = rule::Operator::GreaterEqualThan {
            first: Value::Text("first_arg=".to_owned()),
            second: Value::Text("second_arg".to_owned()),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("ge", operator.name());
    }

    #[test]
    fn build_should_return_the_greater_operator() {
        let ops = rule::Operator::GreaterThan {
            first: Value::Text("first_arg=".to_owned()),
            second: Value::Text("second_arg".to_owned()),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("gt", operator.name());
    }

    #[test]
    fn build_should_return_the_less_equal_operator() {
        let ops = rule::Operator::LessEqualThan {
            first: Value::Text("first_arg=".to_owned()),
            second: Value::Text("second_arg".to_owned()),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("le", operator.name());
    }

    #[test]
    fn build_should_return_the_less_operator() {
        let ops = rule::Operator::LessThan {
            first: Value::Text("first_arg=".to_owned()),
            second: Value::Text("second_arg".to_owned()),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("lt", operator.name());
    }

    #[test]
    fn build_should_return_the_contain_operator() {
        let ops = rule::Operator::Contain {
            first: Value::Text("first_arg=".to_owned()),
            second: Value::Text("second_arg".to_owned()),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("contain", operator.name());
    }

    #[test]
    fn build_should_return_the_regex_operator() {
        let ops =
            rule::Operator::Regex { regex: "[a-fA-F0-9]".to_owned(), target: "target".to_owned() };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("regex", operator.name());
    }

    #[test]
    fn build_should_return_the_and_operator() {
        let ops = rule::Operator::And {
            operators: vec![rule::Operator::Equal {
                first: Value::Text("first_arg".to_owned()),
                second: Value::Text("second_arg".to_owned()),
            }],
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("and", operator.name());
    }

    #[test]
    fn build_should_return_the_or_operator() {
        let ops = rule::Operator::Or { operators: vec![] };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("or", operator.name());
    }

    #[test]
    fn build_should_return_the_true_operator() {
        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &None).unwrap();

        assert_eq!("true", operator.name());
    }
}
