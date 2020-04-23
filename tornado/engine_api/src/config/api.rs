use crate::auth::{AuthContext, Permission};
use crate::error::ApiError;
use tornado_engine_api_dto::common::Id;
use tornado_engine_matcher::config::MatcherConfig;

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
#[async_trait::async_trait]
pub trait ConfigApiHandler: Send + Sync {
    async fn get_current_config(&self) -> Result<MatcherConfig, ApiError>;
    async fn reload_configuration(&self) -> Result<MatcherConfig, ApiError>;
}

pub struct ConfigApi<A: ConfigApiHandler> {
    handler: A
}

impl <A: ConfigApiHandler> ConfigApi<A> {

    pub fn new(handler: A) -> Self {
        Self {
            handler
        }
    }

    /// Returns the current configuration of tornado
    pub async fn get_current_configuration(&self, auth: AuthContext<'_>) -> Result<MatcherConfig, ApiError> {
        auth.has_permission(Permission::ConfigView)?;
        self.handler.get_current_config().await
    }

    /// Returns the list of available drafts
    pub async fn get_drafts(&self, auth: AuthContext<'_>) -> Result<Vec<String>, ApiError> {
        auth.has_permission(Permission::ConfigView)?;
        unimplemented!()
    }

    /// Returns a draft by id
    pub async fn get_draft(
        &self,
        auth: AuthContext<'_>,
        _draft_id: String,
    ) -> Result<MatcherConfig, ApiError> {
        auth.has_permission(Permission::ConfigView)?;
        unimplemented!()
    }

    /// Creats a new draft and returns the id
    pub async fn create_draft(&self, auth: AuthContext<'_>) -> Result<Id<String>, ApiError> {
        auth.has_permission(Permission::ConfigEdit)?;
        unimplemented!()
    }

    /// Update a draft
    pub async fn update_draft(
        &self,
        auth: AuthContext<'_>,
        _draft_id: String,
        _config: MatcherConfig,
    ) -> Result<(), ApiError> {
        auth.has_permission(Permission::ConfigEdit)?;
        unimplemented!()
    }

    /// Deploy a draft by id and reload the tornado configuration
    pub async fn deploy_draft(
        &self,
        auth: AuthContext<'_>,
        _draft_id: String,
    ) -> Result<MatcherConfig, ApiError> {

        // Todo: add deploy logic

        auth.has_permission(Permission::ConfigEdit)?;
        self.handler.reload_configuration().await
    }

    /// Deletes a draft by id
    pub async fn delete_draft(
        &self,
        auth: AuthContext<'_>,
        _draft_id: String,
    ) -> Result<(), ApiError> {
        auth.has_permission(Permission::ConfigEdit)?;
        unimplemented!()
    }
}
