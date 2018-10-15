use std::fmt;
pub mod parser;
pub mod rules;

/// Defines the structure of a generic rule.
pub trait Rule: fmt::Debug {
    /// Returns the Rule name
    fn name(&self) -> &str;

    /// Executes the current rule on a target Event and returns whether the Event matches it.
    fn evaluate(&self) -> bool;
}
