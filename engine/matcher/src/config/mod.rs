use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::error::MatcherError;
use serde::{Deserialize, Serialize};

pub mod filter;
pub mod fs;
pub mod rule;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum MatcherConfig {
    Filter { name: String, filter: Filter, nodes: Vec<MatcherConfig> },
    Ruleset { name: String, rules: Vec<Rule> },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum Defaultable<T: Serialize + Clone> {
    #[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
    Value(T),
    Default {},
}

impl<T: Serialize + Clone> Into<Option<T>> for Defaultable<T> {
    fn into(self) -> Option<T> {
        match self {
            Defaultable::Value(value) => Some(value),
            Defaultable::Default {} => None,
        }
    }
}

/// A MatcherConfigManager permits to read and manipulate the Tornado Configuration
/// from a configuration source.
pub trait MatcherConfigManager: Sync + Send {
    fn read(&self) -> Result<MatcherConfig, MatcherError>;
}
