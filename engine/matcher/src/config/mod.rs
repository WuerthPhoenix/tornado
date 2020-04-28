use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::error::MatcherError;
use serde::{Deserialize, Serialize};

pub mod filter;
pub mod fs;
pub mod rule;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl<T: Serialize + Clone> From<Option<T>> for Defaultable<T> {
    fn from(source: Option<T>) -> Self {
        match source {
            Some(value) => Defaultable::Value(value),
            None => Defaultable::Default {},
        }
    }
}

/// A MatcherConfigReader permits to read and manipulate the Tornado Configuration
/// from a configuration source.
pub trait MatcherConfigReader: Sync + Send {
    fn get_config(&self) -> Result<MatcherConfig, MatcherError>;
}

/// A MatcherConfigEditor permits to edit Tornado Configuration drafts
pub trait MatcherConfigEditor: Sync + Send {
    /// Returns the list of available drafts
    fn get_drafts(&self) -> Result<Vec<String>, MatcherError>;

    /// Returns a draft by id
    fn get_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError>;

    /// Creats a new draft and returns the id
    fn create_draft(&self) -> Result<String, MatcherError>;

    /// Update a draft
    fn update_draft(&self, draft_id: &str, config: &MatcherConfig) -> Result<(), MatcherError>;

    /// Deploy a draft by id replacing the current tornado configuration
    fn deploy_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError>;

    /// Deletes a draft by id
    fn delete_draft(&self, draft_id: &str) -> Result<(), MatcherError>;
}
