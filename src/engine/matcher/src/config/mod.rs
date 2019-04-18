use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::error::MatcherError;
use serde_derive::{Deserialize, Serialize};
use std::collections::btree_map::BTreeMap;

pub mod fs;
pub mod filter;
pub mod rule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatcherConfig {
    Filter { filter: Filter, nodes: BTreeMap<String, MatcherConfig> },
    Rules { rules: Vec<Rule> },
}

pub trait MatcherConfigManager {
    fn read(&self) -> Result<MatcherConfig, MatcherError>;
}

