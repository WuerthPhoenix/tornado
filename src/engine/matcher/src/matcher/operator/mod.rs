use accessor::AccessorBuilder;
use config;
use error::MatcherError;
use model::ProcessedEvent;
use std::fmt;

pub mod and;
pub mod equal;
pub mod contain;
pub mod or;
pub mod regex;

/// Trait for a generic matcher.operator.
pub trait Operator: fmt::Debug + Send + Sync {
    /// Returns the Operator name
    fn name(&self) -> &str;

    /// Executes the current matcher.operator on a target Event and returns whether the Event matches it.
    fn evaluate(&self, event: &ProcessedEvent) -> bool;
}

/// Operator instance builder.
#[derive(Default)]
pub struct OperatorBuilder {
    accessor: AccessorBuilder,
}

impl OperatorBuilder {
    pub fn new() -> OperatorBuilder {
        OperatorBuilder { accessor: AccessorBuilder::new() }
    }

    /// Returns a specific Operator instance based on matcher.operator configuration.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// extern crate tornado_engine_matcher;
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
                Ok(Box::new(::matcher::operator::and::And::build("", &operators, self)?))
            }
            config::Operator::Or { operators } => {
                Ok(Box::new(::matcher::operator::or::Or::build("", &operators, self)?))
            }
            config::Operator::Equal { first, second } => {
                Ok(Box::new(::matcher::operator::equal::Equal::build(
                    self.accessor.build(rule_name, first)?,
                    self.accessor.build(rule_name, second)?,
                )?))
            }
            config::Operator::Contain { text, substring } => {
                Ok(Box::new(::matcher::operator::contain::Contain::build(
                    self.accessor.build(rule_name, text)?,
                    self.accessor.build(rule_name, substring)?,
                )?))
            }
            config::Operator::Regex { regex, target } => {
                Ok(Box::new(::matcher::operator::regex::Regex::build(
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
        assert!(builder.build("", &ops).is_err());
    }

    #[test]
    fn build_should_return_the_equal_operator() {
        let ops = config::Operator::Equal {
            first: "first_arg=".to_owned(),
            second: "second_arg".to_owned(),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build("", &ops).unwrap();

        assert_eq!("equal", operator.name());
    }

    #[test]
    fn build_should_return_the_contain_operator() {
        let ops = config::Operator::Contain {
            text: "first_arg=".to_owned(),
            substring: "second_arg".to_owned(),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build("", &ops).unwrap();

        assert_eq!("contain", operator.name());
    }

    #[test]
    fn build_should_return_the_regex_operator() {
        let ops = config::Operator::Regex {
            regex: "[a-fA-F0-9]".to_owned(),
            target: "target".to_owned(),
        };

        let builder = OperatorBuilder::new();
        let operator = builder.build("", &ops).unwrap();

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
        let operator = builder.build("", &ops).unwrap();

        assert_eq!("and", operator.name());
    }

    #[test]
    fn build_should_return_the_or_operator() {
        let ops = config::Operator::Or { operators: vec![] };

        let builder = OperatorBuilder::new();
        let operator = builder.build("", &ops).unwrap();

        assert_eq!("or", operator.name());
    }

}
