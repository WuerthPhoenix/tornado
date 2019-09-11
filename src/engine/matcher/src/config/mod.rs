use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::error::MatcherError;
use serde_derive::{Deserialize, Serialize};

pub mod filter;
pub mod fs;
pub mod rule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatcherConfig {
    Filter { name: String, filter: Filter, nodes: Vec<MatcherConfig> },
    Ruleset { name: String, rules: Vec<Rule> },
}

/// A MatcherConfigManager permits to read and manipulate the Tornado Configuration
/// from a configuration source.
pub trait MatcherConfigManager: Sync + Send {
    fn read(&self) -> Result<MatcherConfig, MatcherError>;
}
