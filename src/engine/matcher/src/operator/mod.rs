use accessor::AccessorBuilder;
use config;
use error::MatcherError;
use operator;
use std::fmt;
use tornado_common::Event;

pub mod and;
pub mod equal;
pub mod or;
pub mod regex;

/// Defines the structure of a generic operator.
pub trait Operator: fmt::Debug {
    /// Returns the Operator name
    fn name(&self) -> &str;

    /// Executes the current operator on a target Event and returns whether the Event matches it.
    fn evaluate(&self, event: &Event) -> bool;
}

/// Operator instance builder.
#[derive(Default)]
pub struct OperatorBuilder {
    accessor: AccessorBuilder,
}

impl OperatorBuilder {
    pub fn new() -> OperatorBuilder {
        OperatorBuilder {
            accessor: AccessorBuilder::new(),
        }
    }

    /// Returns a specific Operator instance based on operator configuration.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// extern crate tornado_engine_matcher;
    ///
    /// use tornado_engine_matcher::operator::OperatorBuilder;
    /// use tornado_engine_matcher::config;
    ///
    /// let ops = config::Operator::Equals {
    ///              first: "${event.type}".to_owned(),
    ///              second: "email".to_owned(),
    ///           };
    ///
    /// let builder = OperatorBuilder::new();
    /// let operator = builder.build(&ops).unwrap(); // operator is an instance of Equal
    /// ```
    pub fn build(
        &self,
        config: &config::Operator,
    ) -> Result<Box<operator::Operator>, MatcherError> {
        match config {
            config::Operator::Equals { first, second } => {
                Ok(Box::new(operator::equal::Equal::build(
                    self.accessor.build(first)?,
                    self.accessor.build(second)?,
                )?))
            }
            config::Operator::And { operators } => {
                Ok(Box::new(operator::and::And::build(&operators, self)?))
            }
            config::Operator::Or { operators } => {
                Ok(Box::new(operator::or::Or::build(&operators, self)?))
            }
            config::Operator::Regex { regex, target } => Ok(Box::new(
                operator::regex::Regex::build(regex, self.accessor.build(target)?)?,
            )),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn build_should_return_error_if_wrong_operator() {
        let ops = config::Operator::Equals {
            first: "${WRONG_ARG}".to_owned(),
            second: "second_arg".to_owned(),
        };

        let builder = OperatorBuilder::new();
        assert!(builder.build(&ops).is_err());
    }

    #[test]
    fn build_should_return_the_equal_rule() {
        let ops = config::Operator::Equals {
            first: "first_arg=".to_owned(),
            second: "second_arg".to_owned(),
        };

        let builder = OperatorBuilder::new();
        let rule = builder.build(&ops).unwrap();

        assert_eq!("equal", rule.name());
    }

    #[test]
    fn build_should_return_the_regex_rule() {
        let ops = config::Operator::Regex {
            regex: "[a-fA-F0-9]".to_owned(),
            target: "target".to_owned(),
        };

        let builder = OperatorBuilder::new();
        let rule = builder.build(&ops).unwrap();

        assert_eq!("regex", rule.name());
    }

    #[test]
    fn build_should_return_the_and_rule() {
        let ops = config::Operator::And {
            operators: vec![config::Operator::Equals {
                first: "first_arg".to_owned(),
                second: "second_arg".to_owned(),
            }],
        };

        let builder = OperatorBuilder::new();
        let rule = builder.build(&ops).unwrap();

        assert_eq!("and", rule.name());
    }

    #[test]
    fn build_should_return_the_or_rule() {
        let ops = config::Operator::Or { operators: vec![] };

        let builder = OperatorBuilder::new();
        let rule = builder.build(&ops).unwrap();

        assert_eq!("or", rule.name());
    }

}
