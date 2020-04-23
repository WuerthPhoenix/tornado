use crate::auth::{AuthContext, Permission};
use crate::error::ApiError;
use tornado_engine_api_dto::common::Id;
use tornado_engine_matcher::config::MatcherConfig;

#[derive(Default)]
pub struct ConfigApi {}

impl ConfigApi {
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
