use crate::auth::AuthContext;
use crate::error::ApiError;
use tornado_engine_api_dto::common::Id;
use tornado_engine_matcher::config::MatcherConfig;

pub const CONFIG_EDIT_PERMISSION: &str = "config_edit";
pub const CONFIG_VIEW_PERMISSION: &str = "config_view";

pub struct ConfigApi {
    //  auth_service: AuthService
}

impl ConfigApi {
    /*
        pub fn new(auth_service: AuthService) -> Self {
            Self {
                auth_service
            }
        }
    */
    /// Returns the list of available drafts
    pub async fn get_drafts(&self, auth: AuthContext<'_>) -> Result<Vec<String>, ApiError> {
        auth.has_permission(CONFIG_VIEW_PERMISSION)?;
        unimplemented!()
    }

    /// Returns a draft by id
    pub async fn get_draft(
        &self,
        auth: AuthContext<'_>,
        _draft_id: String,
    ) -> Result<MatcherConfig, ApiError> {
        auth.has_permission(CONFIG_VIEW_PERMISSION)?;
        unimplemented!()
    }

    /// Creats a new draft and returns the id
    pub async fn create_draft(&self, auth: AuthContext<'_>) -> Result<Id<String>, ApiError> {
        auth.has_permission(CONFIG_EDIT_PERMISSION)?;
        unimplemented!()
    }

    /// Update a draft
    pub async fn update_draft(
        &self,
        auth: AuthContext<'_>,
        _draft_id: String,
        _config: MatcherConfig,
    ) -> Result<(), ApiError> {
        auth.has_permission(CONFIG_EDIT_PERMISSION)?;
        unimplemented!()
    }

    /// Deletes a draft by id
    pub async fn delete_draft(
        &self,
        auth: AuthContext<'_>,
        _draft_id: String,
    ) -> Result<(), ApiError> {
        auth.has_permission(CONFIG_EDIT_PERMISSION)?;
        unimplemented!()
    }
}
