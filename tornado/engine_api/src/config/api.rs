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
        auth.has_permission(&Permission::ConfigView)?;
        Ok(self.config_manager.get_config()?)
    }

    /// Returns the list of available drafts
    pub async fn get_drafts(&self, auth: AuthContext<'_>) -> Result<Vec<String>, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        Ok(self.config_manager.get_drafts()?)
    }

    /// Returns a draft by id
    pub async fn get_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
    ) -> Result<MatcherConfigDraft, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        self.get_draft_and_check_owner(&auth, draft_id).await
    }

    /// Creats a new draft and returns the id
    pub async fn create_draft(&self, auth: AuthContext<'_>) -> Result<Id<String>, ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        Ok(self.config_manager.create_draft(auth.auth.user).map(|id| Id { id })?)
    }

    /// Update a draft
    pub async fn update_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
        config: MatcherConfig,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        self.get_draft_and_check_owner(&auth, draft_id).await?;
        Ok(self.config_manager.update_draft(draft_id, auth.auth.user, &config)?)
    }

    /// Deploy a draft by id and reload the tornado configuration
    pub async fn deploy_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
    ) -> Result<MatcherConfig, ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        self.get_draft_and_check_owner(&auth, draft_id).await?;
        self.config_manager.deploy_draft(draft_id)?;
        self.handler.reload_configuration().await
    }

    /// Deletes a draft by id
    pub async fn delete_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        self.get_draft_and_check_owner(&auth, draft_id).await?;
        Ok(self.config_manager.delete_draft(draft_id)?)
    }

    pub async fn draft_take_over(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        Ok(self.config_manager.draft_take_over(draft_id, auth.auth.user)?)
    }

    async fn get_draft_and_check_owner(
        &self,
        auth: &AuthContext<'_>,
        draft_id: &str,
    ) -> Result<MatcherConfigDraft, ApiError> {
        let draft = self.config_manager.get_draft(draft_id)?;
        auth.is_owner(&draft)?;
        Ok(draft)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::auth::Permission;
    use crate::error::ApiError;
    use async_trait::async_trait;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use tornado_engine_api_dto::auth::Auth;
    use tornado_engine_matcher::config::{
        MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData,
    };
    use tornado_engine_matcher::error::MatcherError;

    const DRAFT_OWNER_ID: &str = "OWNER";

    struct TestConfigManager {}

    impl MatcherConfigReader for TestConfigManager {
        fn get_config(&self) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![] })
        }
    }

    impl MatcherConfigEditor for TestConfigManager {
        fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
            Ok(vec![])
        }

        fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError> {
            Ok(MatcherConfigDraft {
                data: MatcherConfigDraftData {
                    user: DRAFT_OWNER_ID.to_owned(),
                    draft_id: draft_id.to_owned(),
                    created_ts_ms: 0,
                    updated_ts_ms: 0,
                },
                config: MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![] },
            })
        }

        fn create_draft(&self, _user: String) -> Result<String, MatcherError> {
            Ok("".to_owned())
        }

        fn update_draft(
            &self,
            _draft_id: &str,
            _user: String,
            _config: &MatcherConfig,
        ) -> Result<(), MatcherError> {
            Ok(())
        }

        fn deploy_draft(&self, _draft_id: &str) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset_new".to_owned(), rules: vec![] })
        }

        fn delete_draft(&self, _draft_id: &str) -> Result<(), MatcherError> {
            Ok(())
        }

        fn draft_take_over(&self, _draft_id: &str, _user: String) -> Result<(), MatcherError> {
            Ok(())
        }
    }

    struct TestApiHandler {}

    #[async_trait]
    impl ConfigApiHandler for TestApiHandler {
        async fn reload_configuration(&self) -> Result<MatcherConfig, ApiError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset_new".to_owned(), rules: vec![] })
        }
    }

    fn auth_permissions() -> BTreeMap<Permission, Vec<String>> {
        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigEdit, vec!["edit".to_owned()]);
        permission_roles_map.insert(Permission::ConfigView, vec!["view".to_owned()]);
        permission_roles_map
    }

    fn create_users(
        permissions_map: &BTreeMap<Permission, Vec<String>>,
    ) -> (AuthContext, AuthContext, AuthContext, AuthContext) {
        let not_owner_edit_and_view = AuthContext::new(
            Auth {
                user: "a_user".to_owned(),
                roles: vec!["edit".to_owned(), "view".to_owned()],
                preferences: None,
            },
            permissions_map,
        );

        let owner_view = AuthContext::new(
            Auth {
                user: DRAFT_OWNER_ID.to_owned(),
                roles: vec!["view".to_owned()],
                preferences: None,
            },
            permissions_map,
        );

        let owner_edit = AuthContext::new(
            Auth {
                user: DRAFT_OWNER_ID.to_owned(),
                roles: vec!["edit".to_owned()],
                preferences: None,
            },
            permissions_map,
        );

        let owner_edit_and_view = AuthContext::new(
            Auth {
                user: DRAFT_OWNER_ID.to_owned(),
                roles: vec!["edit".to_owned(), "view".to_owned()],
                preferences: None,
            },
            permissions_map,
        );

        (not_owner_edit_and_view, owner_view, owner_edit, owner_edit_and_view)
    }

    #[actix_rt::test]
    async fn get_current_configuration_should_require_view_permission() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let (not_owner_edit_and_view, owner_view, owner_edit, owner_edit_and_view) =
            create_users(&permissions_map);

        // Act & Assert
        assert!(api.get_current_configuration(not_owner_edit_and_view).await.is_ok());
        assert!(api.get_current_configuration(owner_view).await.is_ok());
        assert!(api.get_current_configuration(owner_edit).await.is_err());
        assert!(api.get_current_configuration(owner_edit_and_view).await.is_ok());
    }

    #[actix_rt::test]
    async fn get_drafts_should_require_view_permission() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let (not_owner_edit_and_view, owner_view, owner_edit, owner_edit_and_view) =
            create_users(&permissions_map);

        // Act & Assert
        assert!(api.get_drafts(not_owner_edit_and_view).await.is_ok());
        assert!(api.get_drafts(owner_view).await.is_ok());
        assert!(api.get_drafts(owner_edit).await.is_err());
        assert!(api.get_drafts(owner_edit_and_view).await.is_ok());
    }

    #[actix_rt::test]
    async fn get_draft_should_require_view_permission_and_owner() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let (not_owner_edit_and_view, owner_view, owner_edit, owner_edit_and_view) =
            create_users(&permissions_map);

        // Act & Assert
        assert!(api.get_draft(not_owner_edit_and_view, "").await.is_err());
        assert!(api.get_draft(owner_view, "").await.is_ok());
        assert!(api.get_draft(owner_edit, "").await.is_err());
        assert!(api.get_draft(owner_edit_and_view, "").await.is_ok());
    }

    #[actix_rt::test]
    async fn create_draft_should_require_edit_permission() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let (not_owner_edit_and_view, owner_view, owner_edit, owner_edit_and_view) =
            create_users(&permissions_map);

        // Act & Assert
        assert!(api.create_draft(not_owner_edit_and_view).await.is_ok());
        assert!(api.create_draft(owner_view).await.is_err());
        assert!(api.create_draft(owner_edit).await.is_ok());
        assert!(api.create_draft(owner_edit_and_view).await.is_ok());
    }

    #[actix_rt::test]
    async fn update_draft_should_require_edit_permission_and_owner() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let (not_owner_edit_and_view, owner_view, owner_edit, owner_edit_and_view) =
            create_users(&permissions_map);

        // Act & Assert
        assert!(api
            .update_draft(
                not_owner_edit_and_view,
                "id",
                MatcherConfig::Ruleset { name: "n".to_owned(), rules: vec![] }
            )
            .await
            .is_err());
        assert!(api
            .update_draft(
                owner_view,
                "id",
                MatcherConfig::Ruleset { name: "n".to_owned(), rules: vec![] }
            )
            .await
            .is_err());
        assert!(api
            .update_draft(
                owner_edit,
                "id",
                MatcherConfig::Ruleset { name: "n".to_owned(), rules: vec![] }
            )
            .await
            .is_ok());
        assert!(api
            .update_draft(
                owner_edit_and_view,
                "id",
                MatcherConfig::Ruleset { name: "n".to_owned(), rules: vec![] }
            )
            .await
            .is_ok());
    }

    #[actix_rt::test]
    async fn delete_draft_should_require_edit_permission_and_owner() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let (not_owner_edit_and_view, owner_view, owner_edit, owner_edit_and_view) =
            create_users(&permissions_map);

        // Act & Assert
        assert!(api.delete_draft(not_owner_edit_and_view, "id").await.is_err());
        assert!(api.delete_draft(owner_view, "id").await.is_err());
        assert!(api.delete_draft(owner_edit, "id").await.is_ok());
        assert!(api.delete_draft(owner_edit_and_view, "id").await.is_ok());
    }

    #[actix_rt::test]
    async fn deploy_draft_should_require_edit_permission_and_owner() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let (not_owner_edit_and_view, owner_view, owner_edit, owner_edit_and_view) =
            create_users(&permissions_map);

        // Act & Assert
        assert!(api.deploy_draft(not_owner_edit_and_view, "id").await.is_err());
        assert!(api.deploy_draft(owner_view, "id").await.is_err());
        assert!(api.deploy_draft(owner_edit, "id").await.is_ok());
        assert!(api.deploy_draft(owner_edit_and_view, "id").await.is_ok());
    }

    #[actix_rt::test]
    async fn draft_take_over_should_require_edit_permission() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let (not_owner_edit_and_view, owner_view, owner_edit, owner_edit_and_view) =
            create_users(&permissions_map);

        // Act & Assert
        assert!(api.draft_take_over(not_owner_edit_and_view, "id").await.is_ok());
        assert!(api.draft_take_over(owner_view, "id").await.is_err());
        assert!(api.draft_take_over(owner_edit, "id").await.is_ok());
        assert!(api.draft_take_over(owner_edit_and_view, "id").await.is_ok());
    }
}
