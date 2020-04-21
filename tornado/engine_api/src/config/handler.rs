use crate::error::ApiError;
use tornado_engine_matcher::config::MatcherConfig;
use crate::auth::{AuthContext};

pub const CONFIG_EDIT_PERMISSION: &str = "config_edit";

pub struct ConfigApiHandler {
  //  auth_service: AuthService
}

impl ConfigApiHandler {
/*
    pub fn new(auth_service: AuthService) -> Self {
        Self {
            auth_service
        }
    }
*/
    /// Returns the list of available drafts
    pub async fn get_drafts(&self, auth: AuthContext<'_>) -> Result<Vec<String>, ApiError> {
        auth.has_permission(CONFIG_EDIT_PERMISSION)?;
        unimplemented!()
    }

    /// Returns a draft by id
    pub async fn get_draft(&self, auth: AuthContext<'_>, _draft_id: String) -> Result<MatcherConfig, ApiError> {
        auth.has_permission(CONFIG_EDIT_PERMISSION)?;
        unimplemented!()
    }

    /// Creats a new draft and returns the id
    pub async fn create_draft(&self, auth: AuthContext<'_>) -> Result<String, ApiError> {
        auth.has_permission(CONFIG_EDIT_PERMISSION)?;
        unimplemented!()
    }

    /// Update a draft
    pub async fn update_draft(&self, auth: AuthContext<'_>, _draft_id: String, _config: MatcherConfig) -> Result<(), ApiError> {
        auth.has_permission(CONFIG_EDIT_PERMISSION)?;
        unimplemented!()
    }

    /// Deletes a draft by id
    pub async fn delete_draft(&self, auth: AuthContext<'_>, _draft_id: String) -> Result<(), ApiError> {
        auth.has_permission(CONFIG_EDIT_PERMISSION)?;
        unimplemented!()
    }

}
