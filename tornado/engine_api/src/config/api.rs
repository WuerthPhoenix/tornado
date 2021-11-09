use crate::auth::auth_v2::AuthContextV2;
use crate::auth::{AuthContext, Permission};
use crate::error::ApiError;
use log::*;
use std::sync::Arc;
use tornado_engine_api_dto::common::Id;
use tornado_engine_api_dto::config::{
    ProcessingTreeNodeConfigDto, ProcessingTreeNodeDetailsDto, TreeInfoDto,
};
use tornado_engine_matcher::config::operation::{matcher_config_filter, NodeFilter};
use tornado_engine_matcher::config::{
    MatcherConfig, MatcherConfigDraft, MatcherConfigEditor, MatcherConfigReader,
};

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
        self.get_authorized_child_nodes(&auth, node_path).await
    }

    async fn get_authorized_child_nodes(
        &self,
        auth: &AuthContextV2<'_>,
        node_path: &str,
    ) -> Result<Vec<ProcessingTreeNodeConfigDto>, ApiError> {
        let filtered_matcher = self.get_filtered_matcher(auth).await?;

        let mut relative_path: Vec<_> =
            if node_path.is_empty() { vec![] } else { node_path.split(',').collect() };

        let mut absolute_path: Vec<_> =
            auth.auth.authorization.path.iter().map(|s| s as &str).collect();

        // We must remove the last element of the authorized path because the endpoint is expected
        // to return the entry point of the authorized tree, when node_path is empty.
        // It is safe to pop from the authorized path because the MatcherConfig is already filtered.
        if absolute_path.pop().is_none() {
            let message = "The authorized node path cannot be empty.";
            warn!("{}", message);
            return Err(ApiError::InvalidAuthorizedPath { message: message.to_owned() });
        };
        absolute_path.append(&mut relative_path);
        let child_nodes = filtered_matcher
            .get_child_nodes_by_path(absolute_path.as_slice())
            .ok_or(ApiError::NodeNotFoundError {
                message: format!("Node for path {:?} not found", absolute_path),
            })?;
        Ok(child_nodes.iter().map(ProcessingTreeNodeConfigDto::from).collect())
    }

    pub async fn get_authorized_tree_info(
        &self,
        auth: &AuthContextV2<'_>,
    ) -> Result<TreeInfoDto, ApiError> {
        auth.has_any_permission(&[&Permission::ConfigView, &Permission::ConfigEdit])?;

        let filtered_matcher = self.get_filtered_matcher(auth).await?;

        let absolute_path: Vec<_> =
            auth.auth.authorization.path.iter().map(|s| s as &str).collect();

        let child_nodes = filtered_matcher
            .get_child_nodes_by_path(absolute_path.as_slice())
            .ok_or(ApiError::NodeNotFoundError {
                message: format!("Node for path {:?} not found", absolute_path),
            })?;

        Ok(Self::fetch_tree_info(child_nodes.as_slice()))
    }

    fn fetch_tree_info(children: &[MatcherConfig]) -> TreeInfoDto {
        children
            .iter()
            .map(|child| match child {
                MatcherConfig::Filter { nodes, .. } => {
                    TreeInfoDto { filters_count: 1, rules_count: 0 } + Self::fetch_tree_info(nodes)
                }
                MatcherConfig::Ruleset { rules, .. } => {
                    TreeInfoDto { filters_count: 0, rules_count: rules.len() }
                }
            })
            .sum()
    }

    async fn get_filtered_matcher(
        &self,
        auth: &AuthContextV2<'_>,
    ) -> Result<MatcherConfig, ApiError> {
        let config = self.config_manager.get_config().await?;
        let node_filter = NodeFilter::map_from(&[auth.auth.authorization.path.clone()]);
        matcher_config_filter(&config, &node_filter).ok_or({
            let message = "The authorized node path does not exist.";
            warn!("{} Path: {:?}", message, &auth.auth.authorization.path);
            ApiError::InvalidAuthorizedPath { message: message.to_owned() }
        })
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
    use tornado_engine_api_dto::auth_v2::{AuthV2, Authorization};
    use tornado_engine_matcher::config::filter::Filter;
    use tornado_engine_matcher::config::rule::{Constraint, Rule};
    use tornado_engine_matcher::config::{
        Defaultable, MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData,
    };
    use tornado_engine_matcher::error::MatcherError;

    const DRAFT_OWNER_ID: &str = "OWNER";

    struct TestConfigManager {}

    #[async_trait::async_trait(?Send)]
    impl MatcherConfigReader for TestConfigManager {
        async fn get_config(&self) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Filter {
                name: "root".to_owned(),
                filter: Filter {
                    description: "".to_string(),
                    active: false,
                    filter: Defaultable::Default {},
                },
                nodes: vec![
                    MatcherConfig::Filter {
                        name: "root_1".to_owned(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
                        nodes: vec![
                            MatcherConfig::Filter {
                                name: "root_1_1".to_owned(),
                                filter: Filter {
                                    description: "".to_string(),
                                    active: false,
                                    filter: Defaultable::Default {},
                                },
                                nodes: vec![],
                            },
                            MatcherConfig::Ruleset {
                                name: "root_1_2".to_string(),
                                rules: vec![Rule {
                                    name: "root_1_2_1".to_string(),
                                    description: "".to_string(),
                                    do_continue: false,
                                    active: true,
                                    constraint: Constraint {
                                        where_operator: None,
                                        with: Default::default(),
                                    },
                                    actions: vec![],
                                }],
                            },
                        ],
                    },
                    MatcherConfig::Filter {
                        name: "root_2".to_owned(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
                        nodes: vec![
                            MatcherConfig::Ruleset {
                                name: "root_2_1".to_string(),
                                rules: vec![Rule {
                                    name: "root_2_1_1".to_string(),
                                    description: "".to_string(),
                                    do_continue: false,
                                    active: true,
                                    constraint: Constraint {
                                        where_operator: None,
                                        with: Default::default(),
                                    },
                                    actions: vec![],
                                }],
                            },
                            MatcherConfig::Ruleset { name: "root_2_2".to_string(), rules: vec![] },
                        ],
                    },
                ],
            })
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
                    roles: vec!["edit".to_owned(), "view".to_owned()],
                },
                preferences: None,
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
        let expected_result = vec![ProcessingTreeNodeConfigDto::Filter {
            name: "root".to_string(),
            rules_count: 2,
            children_count: 2,
            description: "".to_string(),
        }];
        assert_eq!(
            api.get_current_config_processing_tree_nodes_by_path(
                not_owner_edit_and_view,
                &"".to_string()
            )
            .await
            .unwrap(),
            expected_result
        );
        assert_eq!(
            api.get_current_config_processing_tree_nodes_by_path(owner_view, &"".to_string())
                .await
                .unwrap(),
            expected_result
        );
        assert!(api
            .get_current_config_processing_tree_nodes_by_path(owner_edit, &"".to_string())
            .await
            .is_err());
        assert_eq!(
            api.get_current_config_processing_tree_nodes_by_path(
                owner_edit_and_view,
                &"".to_string()
            )
            .await
            .unwrap(),
            expected_result
        );
    }

    #[actix_rt::test]
    async fn get_current_config_processing_tree_nodes_by_empty_path_should_return_authorized_subtree_entry(
    ) {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let user_root_1 = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned(), "root_1".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            &permissions_map,
        );

        // Act
        let res = api
            .get_current_config_processing_tree_nodes_by_path(user_root_1, &"".to_string())
            .await
            .unwrap();

        // Assert
        let expected_result = vec![ProcessingTreeNodeConfigDto::Filter {
            name: "root_1".to_string(),
            rules_count: 1,
            children_count: 2,
            description: "".to_string(),
        }];
        assert_eq!(res, expected_result);
    }

    #[actix_rt::test]
    async fn get_current_config_processing_tree_nodes_by_empty_path_should_return_error_if_authorized_path_does_not_exist(
    ) {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let user_root_3 = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned(), "root_3".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            &permissions_map,
        );

        // Act
        let res = api
            .get_current_config_processing_tree_nodes_by_path(user_root_3, &"".to_string())
            .await;

        // Assert
        assert!(res.is_err());
    }

    #[actix_rt::test]
    async fn get_current_config_processing_tree_nodes_should_return_error_if_authorized_path_is_empty(
    ) {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let user = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization { path: vec![], roles: vec!["view".to_owned()] },
                preferences: None,
            },
            &permissions_map,
        );

        // Act
        let res = api.get_current_config_processing_tree_nodes_by_path(user, &"".to_string()).await;

        // Assert
        assert!(res.is_err());
    }

    #[actix_rt::test]
    async fn get_current_config_processing_tree_nodes_should_return_error_if_authorized_path_root_does_not_exist(
    ) {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let user = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["non_existing".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            &permissions_map,
        );

        // Act
        let res = api.get_current_config_processing_tree_nodes_by_path(user, &"".to_string()).await;

        // Assert
        assert!(res.is_err());
    }

    #[actix_rt::test]
    async fn get_current_config_processing_tree_nodes_by_path_should_start_from_authorized_path() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let user_root_1 = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned(), "root_1".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            &permissions_map,
        );

        // Act
        let res = api
            .get_current_config_processing_tree_nodes_by_path(user_root_1, &"root_1".to_string())
            .await
            .unwrap();

        // Assert
        assert_eq!(
            res,
            vec![
                ProcessingTreeNodeConfigDto::Filter {
                    name: "root_1_1".to_string(),
                    rules_count: 0,
                    children_count: 0,
                    description: "".to_string(),
                },
                ProcessingTreeNodeConfigDto::Ruleset {
                    name: "root_1_2".to_string(),
                    rules_count: 1,
                },
            ]
        );
    }

    #[actix_rt::test]
    async fn get_current_config_processing_tree_nodes_by_path_should_return_error_if_authorized_path_does_not_exist(
    ) {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let user_root_3 = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned(), "root_3".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            &permissions_map,
        );

        // Act & Assert
        assert!(api
            .get_current_config_processing_tree_nodes_by_path(user_root_3, &"root".to_string())
            .await
            .is_err());
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

    #[actix_rt::test]
    async fn get_tree_info_should_return_aggregate_number_of_filters_and_rules() {
        // Arrange
        let test = TestConfigManager {}.get_config().await.unwrap();
        let root = test.get_child_nodes_by_path(&[]).unwrap();

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(&root);

        // Assert
        let expected = TreeInfoDto { rules_count: 2, filters_count: 4 };

        assert_eq!(result, expected);
    }

    #[actix_rt::test]
    async fn get_tree_info_should_work_on_filter_without_children() {
        // Arrange
        let test = MatcherConfig::Filter {
            name: "root".to_owned(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![],
        };

        let root = test.get_child_nodes_by_path(&[]).unwrap();

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(&root);

        // Assert
        let expected = TreeInfoDto { rules_count: 0, filters_count: 1 };

        assert_eq!(result, expected);
    }

    #[actix_rt::test]
    async fn get_tree_info_should_work_on_ruleset_as_only_node() {
        // Arrange
        let test = MatcherConfig::Ruleset {
            name: "root".to_owned(),
            rules: vec![Rule {
                name: "".to_string(),
                description: "".to_string(),
                do_continue: false,
                active: false,
                constraint: Constraint { where_operator: None, with: Default::default() },
                actions: vec![],
            }],
        };

        let root = test.get_child_nodes_by_path(&[]).unwrap();

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(&root);

        // Assert
        let expected = TreeInfoDto { rules_count: 1, filters_count: 0 };

        assert_eq!(result, expected);
    }

    #[actix_rt::test]
    async fn get_tree_info_should_work_on_empty_config() {
        // Arrange
        let root = [];

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(&root);

        // Assert
        let expected = TreeInfoDto { rules_count: 0, filters_count: 0 };

        assert_eq!(result, expected);
    }

    #[actix_rt::test]
    async fn get_authorized_tree_info_should_require_permissions() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let user1 = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            &permissions_map,
        );
        let user2 = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization { path: vec!["root".to_owned()], roles: vec![] },
                preferences: None,
            },
            &permissions_map,
        );

        // Act
        let res1 = api.get_authorized_tree_info(&user1).await;
        let res2 = api.get_authorized_tree_info(&user2).await;

        // Assert
        assert!(res1.is_ok());
        assert!(matches!(res2, Err(ApiError::ForbiddenError { .. })));
    }
}
