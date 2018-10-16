use std::fmt;
use tornado_common::Event;

pub mod and;
pub mod equal;
pub mod or;
pub mod parser;
pub mod regex;

/// Defines the structure of a generic operator.
pub trait Operator: fmt::Debug {
    /// Returns the Operator name
    fn name(&self) -> &str;

    /// Executes the current operator on a target Event and returns whether the Event matches it.
    fn evaluate(&self, event: &Event) -> bool;
}
