use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::error::MatcherError;
use serde_derive::{Deserialize, Serialize};
use std::collections::btree_map::BTreeMap;

pub mod filter;
pub mod fs;
pub mod rule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatcherConfig {
    Filter { filter: Filter, nodes: BTreeMap<String, MatcherConfig> },
    Rules { rules: Vec<Rule> },
}

/// A MatcherConfigManager permits to read and manipulate the Tornado Configuration
/// from a configuration source.
pub trait MatcherConfigManager: Sync + Send {
    fn read(&self) -> Result<MatcherConfig, MatcherError>;
}
