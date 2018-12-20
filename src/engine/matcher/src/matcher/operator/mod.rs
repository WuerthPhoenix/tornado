use crate::accessor::AccessorBuilder;
use crate::config;
use crate::error::MatcherError;
use crate::model::ProcessedEvent;
use log::*;
use std::fmt;

pub mod and;
pub mod contain;
pub mod equal;
pub mod or;
pub mod regex;
pub mod true_operator;

/// The Trait for a generic matcher.operator
pub trait Operator: fmt::Debug + Send + Sync {
    /// Returns the Operator name.
    fn name(&self) -> &str;

    /// Executes the current matcher.operator on a target Event and returns whether the Event matches it.
    fn evaluate(&self, event: &ProcessedEvent) -> bool;
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
        config: &Option<config::Operator>,
    ) -> Result<Box<Operator>, MatcherError> {
        let result: Result<Box<Operator>, MatcherError> = match config {
            Some(operator) => self.build(rule_name, operator),
            None => Ok(Box::new(crate::matcher::operator::true_operator::True {})),
        };

        info!(
            "OperatorBuilder - build: return matcher.operator [{:?}] for input value [{:?}]",
            &result, config
        );
        result
    }

    /// Returns a specific Operator instance based on the matcher.operator configuration.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// use tornado_engine_matcher::matcher::operator::OperatorBuilder;
    /// use tornado_engine_matcher::config;
    ///
    /// let ops = config::Operator::Equal {
    ///              first: "${event.type}".to_owned(),
    ///              second: "email".to_owned(),
    ///           };
    ///
    /// let builder = OperatorBuilder::new();
    /// let operator = builder.build("rule_name", &ops).unwrap(); // operator is an instance of Equal
    /// ```
    pub fn build(
        &self,
        rule_name: &str,
        config: &config::Operator,
    ) -> Result<Box<Operator>, MatcherError> {
        let result: Result<Box<Operator>, MatcherError> = match config {
            config::Operator::And { operators } => {
                Ok(Box::new(crate::matcher::operator::and::And::build("", &operators, self)?))
            }
            config::Operator::Or { operators } => {
                Ok(Box::new(crate::matcher::operator::or::Or::build("", &operators, self)?))
            }
            config::Operator::Equal { first, second } => {
                Ok(Box::new(crate::matcher::operator::equal::Equal::build(
                    self.accessor.build(rule_name, first)?,
                    self.accessor.build(rule_name, second)?,
                )?))
            }
            config::Operator::Contain { text, substring } => {
                Ok(Box::new(crate::matcher::operator::contain::Contain::build(
                    self.accessor.build(rule_name, text)?,
                    self.accessor.build(rule_name, substring)?,
                )?))
            }
            config::Operator::Regex { regex, target } => {
                Ok(Box::new(crate::matcher::operator::regex::Regex::build(
                    regex,
                    self.accessor.build(rule_name, target)?,
                )?))
            }
        };

        info!(
            "OperatorBuilder - build: return matcher.operator [{:?}] for input value [{:?}]",
            &result, config
        );
        result
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn build_should_return_error_if_wrong_operator() {
        let ops = config::Operator::Equal {
            first: "${WRONG_ARG}".to_owned(),
            second: "second_arg".to_owned(),
        };

        let builder = OperatorBuilder::new();
        assert!(builder.build_option("", &Some(ops)).is_err());
    }

    #[test]
    fn build_should_return_the_equal_operator() {
        let ops = config::Operator::Equal {
            first: "first_arg=".to_owned(),
            second: "second_arg".to_owned(),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("equal", operator.name());
    }

    #[test]
    fn build_should_return_the_contain_operator() {
        let ops = config::Operator::Contain {
            text: "first_arg=".to_owned(),
            substring: "second_arg".to_owned(),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("contain", operator.name());
    }

    #[test]
    fn build_should_return_the_regex_operator() {
        let ops = config::Operator::Regex {
            regex: "[a-fA-F0-9]".to_owned(),
            target: "target".to_owned(),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("regex", operator.name());
    }

    #[test]
    fn build_should_return_the_and_operator() {
        let ops = config::Operator::And {
            operators: vec![config::Operator::Equal {
                first: "first_arg".to_owned(),
                second: "second_arg".to_owned(),
            }],
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build_option("", &Some(ops)).unwrap();

        assert_eq!("and", operator.name());
    }

    #[test]
    fn build_should_return_the_or_operator() {
        let ops = config::Operator::Or { operators: vec![] };

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
