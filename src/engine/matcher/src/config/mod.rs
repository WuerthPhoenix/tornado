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

impl MatcherConfig {

    pub fn name(&self) -> &str {
        match self {
            MatcherConfig::Filter{name, ..} => name,
            MatcherConfig::Ruleset{name, ..} => name,
        }
    }

    pub fn get_config(name: &str, nodes: &[MatcherConfig]) -> Option<&MatcherConfig> {
        for node in nodes {
            if node.name().eq(name) {
                return Some(node);
            }
        };
        None
    }

    pub fn get_rule(name: &str, rules: &[Rule]) -> Option<&Rule> {
        for rule in rules {
            if rule.name.eq(name) {
                return Some(rule);
            }
        };
        None
    }
}

/// A MatcherConfigManager permits to read and manipulate the Tornado Configuration
/// from a configuration source.
pub trait MatcherConfigManager: Sync + Send {
    fn read(&self) -> Result<MatcherConfig, MatcherError>;
}
