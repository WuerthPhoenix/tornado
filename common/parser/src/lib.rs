mod interpolator;
mod parser;

use lazy_static::lazy_static;
use regex::{Match, Regex};
use serde_json::Value;
use std::borrow::Cow;
use std::fmt::Debug;
use tornado_common_types::ValueGet;

pub use crate::parser::{Parser, ParserBuilder, ParserError, EXTRACTED_VARIABLES_KEY};

pub const EXPRESSION_START_DELIMITER: &str = "${";
pub const EXPRESSION_END_DELIMITER: &str = "}";
pub const FOREACH_ITEM_KEY: &str = "item";
pub const EVENT_KEY: &str = "event";

// This regex is used to match the `${event.something}` pattern.
// It literally matches the sequence `${` followed by one or more chars followed by `}`.
lazy_static! {
    static ref RE: Regex = Regex::new(r"(\$\{[^}]+})").expect("Accessor regex must be valid");
}

pub struct Template<'template> {
    template_string: &'template str,
    matches: Vec<Match<'template>>,
}

impl<'template_string> From<&'template_string str> for Template<'template_string> {
    fn from(template_string: &'template_string str) -> Self {
        let matches = RE.find_iter(template_string).collect();
        Self { template_string, matches }
    }
}

impl Template<'_> {
    pub fn template_string(&self) -> &str {
        self.template_string
    }

    pub fn matches(&self) -> &[Match] {
        self.matches.as_slice()
    }

    pub fn is_accessor(&self) -> bool {
        self.matches.len() == 1 && self.matches[0].as_str() == self.template_string
    }

    /// Returns whether this template requires interpolation.
    /// This is true only if the template contains at least both a static part (e.g. constant text)
    /// and a dynamic part (e.g. placeholders to be resolved at runtime).
    /// When the interpolator is not required, it can be replaced by a simpler Accessor.
    pub fn is_interpolator(&self) -> bool {
        !self.matches.is_empty() && !self.is_accessor()
    }
}

pub trait CustomParser: Sync + Send + Debug {
    fn parse_value<'o>(&'o self, value: &'o Value, context: &str) -> Option<Cow<'o, Value>>;
}

#[derive(PartialEq, Debug, Clone)]
pub enum ValueGetter {
    Map { key: String },
    Array { index: usize },
}

impl ValueGetter {
    pub fn get<'o, I: ValueGet>(&self, value: &'o I) -> Option<&'o Value> {
        match self {
            ValueGetter::Map { key } => value.get_from_map(key),
            ValueGetter::Array { index } => value.get_from_array(*index),
        }
    }
}

impl From<&str> for ValueGetter {
    fn from(key: &str) -> Self {
        ValueGetter::Map { key: key.to_owned() }
    }
}

impl From<usize> for ValueGetter {
    fn from(index: usize) -> Self {
        ValueGetter::Array { index }
    }
}

pub fn is_valid_matcher_root(keys: &[ValueGetter]) -> bool {
    match keys {
        [ValueGetter::Map { key }, ..] if key == FOREACH_ITEM_KEY => true,
        [ValueGetter::Map { key }] | [ValueGetter::Map { key }, ValueGetter::Map { .. }, ..]
            if key == EVENT_KEY =>
        {
            true
        }
        [ValueGetter::Map { key }, ValueGetter::Map { .. }, ..]
            if key == EXTRACTED_VARIABLES_KEY =>
        {
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_template_is_not_an_accessor() {
        let template = Template::from("");
        assert!(!template.is_accessor());
    }

    #[test]
    fn wrong_syntax_template_is_not_an_accessor() {
        let template = Template::from("$}sadf}");
        assert!(!template.is_accessor());
    }

    #[test]
    fn interpolator_template_is_not_an_accessor() {
        let template = Template::from("${sadf} sdfhj");
        assert!(!template.is_accessor());
    }

    #[test]
    fn accessor_template_is_not_an_accessor() {
        let template = Template::from("${sadf}");
        assert!(template.is_accessor());
    }
}
