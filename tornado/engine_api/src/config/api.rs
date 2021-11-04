use crate::auth::{AuthContext, Permission};
use crate::error::ApiError;
use std::sync::Arc;
use tornado_engine_api_dto::common::Id;
use tornado_engine_api_dto::config::{ProcessingTreeNodeConfigDto, ProcessingTreeNodeDetailsDto};
use tornado_engine_matcher::config::{
    MatcherConfig, MatcherConfigDraft, MatcherConfigEditor, MatcherConfigReader,
};
use crate::auth::auth_v2::AuthContextV2;

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
#[async_trait::async_trait(?Send)]
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
        Ok(self.config_manager.get_config().await?)
    }

    /// Returns child processing tree nodes of a node found by a path
    /// of the current configuration of tornado
    pub async fn get_current_config_processing_tree_nodes_by_path(
        &self,
        auth: AuthContextV2<'_>,
        node_path: &str,
    ) -> Result<Vec<ProcessingTreeNodeConfigDto>, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;

        let config = self.config_manager.get_config().await?;
        let path: Vec<_> = node_path.split(',').collect();

        if let Some(child_nodes) = config.get_child_nodes_by_path(path.as_slice()) {
            let result =
                child_nodes.iter().map(|node| ProcessingTreeNodeConfigDto::from(*node)).collect();
            Ok(result)
        } else {
            Err(ApiError::NodeNotFoundError {
                message: format!("Node for path {} not found", node_path),
            })
        }
    }

    /// Returns processing tree node details by path
    /// in the current configuration of tornado
    pub async fn get_current_config_node_details_by_path(
        &self,
        auth: AuthContextV2<'_>,
        node_path: &str,
    ) -> Result<ProcessingTreeNodeDetailsDto, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;

        let config = self.config_manager.get_config().await?;
        let path: Vec<_> = node_path.split(',').collect();

        if let Some(node) = config.get_node_by_path(path.as_slice()) {
            let result = ProcessingTreeNodeDetailsDto::from(node);
            Ok(result)
        } else {
            Err(ApiError::NodeNotFoundError {
                message: format!("Node for path {} not found", node_path),
            })
        }
    }

    /// Returns the list of available drafts
    pub async fn get_drafts(&self, auth: AuthContext<'_>) -> Result<Vec<String>, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        Ok(self.config_manager.get_drafts().await?)
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
        Ok(self.config_manager.create_draft(auth.auth.user).await.map(|id| Id { id })?)
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
        Ok(self.config_manager.update_draft(draft_id, auth.auth.user, &config).await?)
    }

    /// Deploy a draft by id and reload the tornado configuration
    pub async fn deploy_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
    ) -> Result<MatcherConfig, ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        self.get_draft_and_check_owner(&auth, draft_id).await?;
        self.config_manager.deploy_draft(draft_id).await?;
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
        Ok(self.config_manager.delete_draft(draft_id).await?)
    }

    pub async fn draft_take_over(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        Ok(self.config_manager.draft_take_over(draft_id, auth.auth.user).await?)
    }

    async fn get_draft_and_check_owner(
        &self,
        auth: &AuthContext<'_>,
        draft_id: &str,
    ) -> Result<MatcherConfigDraft, ApiError> {
        let draft = self.config_manager.get_draft(draft_id).await?;
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
    use tornado_engine_api_dto::auth_v2::{AuthV2, Authorization};

    const DRAFT_OWNER_ID: &str = "OWNER";

    struct TestConfigManager {}

    #[async_trait::async_trait(?Send)]
    impl MatcherConfigReader for TestConfigManager {
        async fn get_config(&self) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![] })
        }
    }

    #[async_trait::async_trait(?Send)]
    impl MatcherConfigEditor for TestConfigManager {
        async fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
            Ok(vec![])
        }

        async fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError> {
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

        async fn create_draft(&self, _user: String) -> Result<String, MatcherError> {
            Ok("".to_owned())
        }

        async fn update_draft(
            &self,
            _draft_id: &str,
            _user: String,
            _config: &MatcherConfig,
        ) -> Result<(), MatcherError> {
            Ok(())
        }

        async fn deploy_draft(&self, _draft_id: &str) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset_new".to_owned(), rules: vec![] })
        }

        async fn delete_draft(&self, _draft_id: &str) -> Result<(), MatcherError> {
            Ok(())
        }

        async fn draft_take_over(
            &self,
            _draft_id: &str,
            _user: String,
        ) -> Result<(), MatcherError> {
            Ok(())
        }

        async fn deploy_config(
            &self,
            _config: &MatcherConfig,
        ) -> Result<MatcherConfig, MatcherError> {
            unimplemented!()
        }
    }

    struct TestApiHandler {}

    #[async_trait(?Send)]
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

    fn create_users_v2(
        permissions_map: &BTreeMap<Permission, Vec<String>>,
    ) -> (AuthContextV2, AuthContextV2, AuthContextV2, AuthContextV2) {
        let not_owner_edit_and_view = AuthContextV2::new(
            AuthV2 {
                user: "a_user".to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["edit".to_owned(), "view".to_owned()]
                },
                preferences: None
            },
            permissions_map,
        );

        let owner_view = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            permissions_map,
        );

        let owner_edit = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["edit".to_owned()],
                },
                preferences: None,
            },
            permissions_map,
        );

        let owner_edit_and_view = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["edit".to_owned(), "view".to_owned()],
                },
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

    #[actix_rt::test]
    async fn get_current_config_processing_tree_nodes_by_path_should_require_view_permission() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let (not_owner_edit_and_view, owner_view, owner_edit, owner_edit_and_view) =
            create_users_v2(&permissions_map);

        // Act & Assert
        assert!(api
            .get_current_config_processing_tree_nodes_by_path(
                not_owner_edit_and_view,
                &"".to_string()
            )
            .await
            .is_ok());
        assert!(api
            .get_current_config_processing_tree_nodes_by_path(owner_view, &"".to_string())
            .await
            .is_ok());
        assert!(api
            .get_current_config_processing_tree_nodes_by_path(owner_edit, &"".to_string())
            .await
            .is_err());
        assert!(api
            .get_current_config_processing_tree_nodes_by_path(owner_edit_and_view, &"".to_string())
            .await
            .is_ok());
    }

    #[actix_rt::test]
    async fn get_current_config_node_details_by_path_should_require_view_permission() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let (not_owner_edit_and_view, owner_view, owner_edit, owner_edit_and_view) =
            create_users_v2(&permissions_map);

        // Act & Assert
        assert!(!matches!(
            api.get_current_config_node_details_by_path(
                not_owner_edit_and_view,
                &"root".to_string()
            )
            .await,
            Err(ApiError::ForbiddenError { .. })
        ));
        assert!(!matches!(
            api.get_current_config_node_details_by_path(owner_view, &"root".to_string()).await,
            Err(ApiError::ForbiddenError { .. })
        ));
        assert!(matches!(
            api.get_current_config_node_details_by_path(owner_edit, &"root".to_string()).await,
            Err(ApiError::ForbiddenError { .. })
        ));
        assert!(!matches!(
            api.get_current_config_node_details_by_path(owner_edit_and_view, &"root".to_string())
                .await,
            Err(ApiError::ForbiddenError { .. })
        ));
    }
}
