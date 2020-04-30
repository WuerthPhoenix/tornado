use crate::auth::{AuthContext, Permission};
use crate::error::ApiError;
use std::sync::Arc;
use tornado_engine_api_dto::common::Id;
use tornado_engine_matcher::config::{
    MatcherConfig, MatcherConfigDraft, MatcherConfigEditor, MatcherConfigReader,
};

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
#[async_trait::async_trait]
pub trait ConfigApiHandler: Send + Sync {
    async fn reload_configuration(&self) -> Result<MatcherConfig, ApiError>;
}

pub struct ConfigApi<A: ConfigApiHandler, CM: MatcherConfigReader + MatcherConfigEditor> {
    handler: A,
    config_manager: Arc<CM>,
}

impl<A: ConfigApiHandler, CM: MatcherConfigReader + MatcherConfigEditor> ConfigApi<A, CM> {
    pub fn new(handler: A, config_manager: Arc<CM>) -> Self {
        Self { handler, config_manager }
    }

    /// Returns the current configuration of tornado
    pub async fn get_current_configuration(
        &self,
        auth: AuthContext<'_>,
    ) -> Result<MatcherConfig, ApiError> {
        auth.has_permission(Permission::ConfigView)?;
        Ok(self.config_manager.get_config()?)
    }

    /// Returns the list of available drafts
    pub async fn get_drafts(&self, auth: AuthContext<'_>) -> Result<Vec<String>, ApiError> {
        auth.has_permission(Permission::ConfigView)?;
        Ok(self.config_manager.get_drafts()?)
    }

    /// Returns a draft by id
    pub async fn get_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
    ) -> Result<MatcherConfigDraft, ApiError> {
        auth.has_permission(Permission::ConfigView)?;
        Ok(self.config_manager.get_draft(draft_id)?)
    }

    /// Creats a new draft and returns the id
    pub async fn create_draft(&self, auth: AuthContext<'_>) -> Result<Id<String>, ApiError> {
        auth.has_permission(Permission::ConfigEdit)?;
        Ok(self.config_manager.create_draft(auth.auth.user).map(|id| Id { id })?)
    }

    /// Update a draft
    pub async fn update_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
        config: MatcherConfig,
    ) -> Result<(), ApiError> {
        auth.has_permission(Permission::ConfigEdit)?;
        Ok(self.config_manager.update_draft(draft_id, auth.auth.user, &config)?)
    }

    /// Deploy a draft by id and reload the tornado configuration
    pub async fn deploy_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
    ) -> Result<MatcherConfig, ApiError> {
        auth.has_permission(Permission::ConfigEdit)?;
        self.config_manager.deploy_draft(draft_id)?;
        self.handler.reload_configuration().await
    }

    /// Deletes a draft by id
    pub async fn delete_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
    ) -> Result<(), ApiError> {
        auth.has_permission(Permission::ConfigEdit)?;
        Ok(self.config_manager.delete_draft(draft_id)?)
    }
}
