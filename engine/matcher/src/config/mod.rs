use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::error::MatcherError;
use serde::{Deserialize, Serialize};

pub mod filter;
pub mod fs;
pub mod rule;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MatcherConfigDraft {
    pub data: MatcherConfigDraftData,
    pub config: MatcherConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MatcherConfigDraftData {
    pub created_ts_ms: i64,
    pub updated_ts_ms: i64,
    pub user: String,
    pub draft_id: String,
}

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

impl <T: Serialize + Clone> From<Defaultable<T>> for Option<T> {
    fn from(default: Defaultable<T>) -> Self {
        match default {
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
#[async_trait::async_trait(?Send)]
pub trait MatcherConfigReader: Sync + Send {
    async fn get_config(&self) -> Result<MatcherConfig, MatcherError>;
}

/// A MatcherConfigEditor permits to edit Tornado Configuration drafts
#[async_trait::async_trait(?Send)]
pub trait MatcherConfigEditor: Sync + Send {
    /// Returns the list of available drafts
    async fn get_drafts(&self) -> Result<Vec<String>, MatcherError>;

    /// Returns a draft by id
    async fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError>;

    /// Creates a new draft and returns the id
    async fn create_draft(&self, user: String) -> Result<String, MatcherError>;

    /// Update a draft
    async fn update_draft(
        &self,
        draft_id: &str,
        user: String,
        config: &MatcherConfig,
    ) -> Result<(), MatcherError>;

    /// Deploy a draft by id replacing the current tornado configuration
    async fn deploy_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError>;

    /// Deletes a draft by id
    async fn delete_draft(&self, draft_id: &str) -> Result<(), MatcherError>;

    /// Sets the ownership of a draft to a user
    async fn draft_take_over(&self, draft_id: &str, user: String) -> Result<(), MatcherError>;

    /// Deploys a new configuration overriding the current one
    async fn deploy_config(&self, config: &MatcherConfig) -> Result<MatcherConfig, MatcherError>;
}
