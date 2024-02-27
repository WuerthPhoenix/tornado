use crate::auth::middleware::{AuthorizedPath, ConfigEdit, ConfigView};
use crate::auth::{AuthContext, AuthContextTrait, Permission};
use crate::config::convert::{dto_into_rule, rule_into_dto};
use crate::error::ApiError;
use std::sync::Arc;
use tornado_engine_api_dto::common::Id;
use tornado_engine_api_dto::config::{
    ProcessingTreeNodeConfigDto, ProcessingTreeNodeDetailsDto, RuleDto, TreeInfoDto,
};
use tornado_engine_matcher::config::{
    MatcherConfig, MatcherConfigDraft, MatcherConfigEditor, MatcherConfigReader,
};

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
#[async_trait::async_trait(? Send)]
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
        auth: &AuthorizedPath<ConfigView>,
    ) -> Result<Vec<ProcessingTreeNodeConfigDto>, ApiError> {
        let config = self.config_manager.get_config().await?;
        self.get_authorized_child_nodes(auth, &config).await
    }

    async fn get_authorized_child_nodes(
        &self,
        auth: &AuthorizedPath<ConfigView>,
        config: &MatcherConfig,
    ) -> Result<Vec<ProcessingTreeNodeConfigDto>, ApiError> {
        let path = auth.path();
        let Some(child_nodes) = config.get_child_nodes_by_path(&path) else {
            return Err(ApiError::NodeNotFoundError);
        };
        Ok(child_nodes.iter().map(ProcessingTreeNodeConfigDto::from).collect())
    }

    pub async fn get_authorized_tree_info(
        &self,
        auth: &AuthorizedPath<ConfigView>,
    ) -> Result<TreeInfoDto, ApiError> {
        let config = &self.config_manager.get_config().await?;
        let Some(config) = config.get_node_by_path(&auth.path()) else {
            return Err(ApiError::NodeNotFoundError);
        };

        Ok(Self::fetch_tree_info(config))
    }

    fn fetch_tree_info(config: &MatcherConfig) -> TreeInfoDto {
        match config {
            MatcherConfig::Filter { nodes, .. } => {
                TreeInfoDto { rules_count: 0, filters_count: 1 }
                    + nodes.iter().map(Self::fetch_tree_info).sum()
            }
            MatcherConfig::Ruleset { rules, .. } => {
                TreeInfoDto { rules_count: rules.len(), filters_count: 0 }
            }
        }
    }

    pub async fn get_current_config_node_details_by_path(
        &self,
        auth: &AuthorizedPath<ConfigView>,
    ) -> Result<ProcessingTreeNodeDetailsDto, ApiError> {
        let config = &self.config_manager.get_config().await?;
        self.get_node_details(auth, config).await
    }

    pub async fn export_draft_tree_starting_from_node_path(
        &self,
        auth: &AuthorizedPath<ConfigView>,
        draft_id: &str,
    ) -> Result<MatcherConfig, ApiError> {
        let draft_config = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft_config)?;
        match draft_config.config.get_node_by_path(&auth.path()) {
            Some(node) => Ok(node.to_owned()),
            None => Err(ApiError::NodeNotFoundError),
        }
    }

    pub async fn get_draft_config_node_details_by_path(
        &self,
        auth: &AuthorizedPath<ConfigView>,
        draft_id: &str,
    ) -> Result<ProcessingTreeNodeDetailsDto, ApiError> {
        let draft_config = &self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(draft_config)?;
        self.get_node_details(auth, &draft_config.config).await
    }

    async fn get_node_details(
        &self,
        auth: &AuthorizedPath<ConfigView>,
        config: &MatcherConfig,
    ) -> Result<ProcessingTreeNodeDetailsDto, ApiError> {
        match config.get_node_by_path(&auth.path()) {
            Some(node) => Ok(ProcessingTreeNodeDetailsDto::from(node)),
            None => Err(ApiError::NodeNotFoundError),
        }
    }

    /// Returns processing tree node details by path
    /// in the current configuration of tornado
    pub async fn get_current_rule_details_by_path(
        &self,
        auth: &AuthorizedPath<ConfigView>,
        rule_name: &str,
    ) -> Result<RuleDto, ApiError> {
        let config = &self.config_manager.get_config().await?;
        self.get_rule_details(auth, config, rule_name).await
    }

    /// Returns processing tree node details by path
    /// in the current configuration of tornado
    pub async fn get_draft_rule_details_by_path(
        &self,
        auth: &AuthorizedPath<ConfigView>,
        draft_id: &str,
        rule_name: &str,
    ) -> Result<RuleDto, ApiError> {
        let draft_config = self.get_draft_and_check_owner(&auth, draft_id).await?;
        self.get_rule_details(auth, &draft_config.config, rule_name).await
    }

    pub async fn create_draft_rule_details_by_path(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
        draft_id: &str,
        rule_dto: RuleDto,
    ) -> Result<(), ApiError> {
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        let rule = dto_into_rule(rule_dto)?;
        draft.config.create_rule(&auth.path(), rule)?;
        Ok(self.config_manager.update_draft(draft_id, auth.user(), &draft.config).await?)
    }

    pub async fn edit_draft_rule_details_by_path(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
        draft_id: &str,
        rule_name: &str,
        rule_dto: RuleDto,
    ) -> Result<(), ApiError> {
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        let rule = dto_into_rule(rule_dto)?;
        draft.config.edit_rule(&auth.path(), rule_name, rule)?;
        Ok(self.config_manager.update_draft(draft_id, auth.user(), &draft.config).await?)
    }

    pub async fn move_draft_rule_by_path(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
        draft_id: &str,
        rule_name: &str,
        position: usize,
    ) -> Result<(), ApiError> {
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        draft.config.move_rule(&auth.path(), rule_name, position)?;
        Ok(self.config_manager.update_draft(draft_id, auth.user(), &draft.config).await?)
    }

    pub async fn delete_draft_rule_details_by_path(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
        draft_id: &str,
        rule_name: &str,
    ) -> Result<(), ApiError> {
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        draft.config.delete_rule(&auth.path(), rule_name)?;
        Ok(self.config_manager.update_draft(draft_id, auth.user(), &draft.config).await?)
    }

    async fn get_rule_details(
        &self,
        auth: &AuthorizedPath<ConfigView>,
        config: &MatcherConfig,
        rule_name: &str,
    ) -> Result<RuleDto, ApiError> {
        let Some(node) = config.get_node_by_path(&auth.path()) else {
            return Err(ApiError::NodeNotFoundError);
        };

        match node {
            MatcherConfig::Filter { .. } => Err(ApiError::NodeNotFoundError),
            MatcherConfig::Ruleset { name: _, rules } => {
                let rule = rules.iter().find(|rule| rule.name == rule_name);
                if let Some(rule) = rule.cloned() {
                    rule_into_dto(rule).map_err(|err| ApiError::InternalServerError {
                        cause: format!("Couldn't convert rule into dto. Error: {}", err),
                    })
                } else {
                    Err(ApiError::NodeNotFoundError)
                }
            }
        }
    }

    /// Returns child processing tree nodes of a node found by a path
    /// of the draft configuration of tornado
    pub async fn get_draft_config_processing_tree_nodes_by_path(
        &self,
        auth: &AuthorizedPath<ConfigView>,
        draft_id: &str,
    ) -> Result<Vec<ProcessingTreeNodeConfigDto>, ApiError> {
        let draft_config = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft_config)?;
        let config = &draft_config.config;
        self.get_authorized_child_nodes(auth, config).await
    }

    /// Returns the list of available drafts
    pub async fn get_drafts(&self, auth: AuthContext<'_>) -> Result<Vec<String>, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        Ok(self.config_manager.get_drafts().await?)
    }

    /// Returns the list of available drafts for a specific tenant
    /// TODO: implement the multitenancy https://siwuerthphoenix.atlassian.net/browse/NEPROD-1232
    pub async fn get_drafts_by_tenant(
        &self,
        _auth: &AuthorizedPath<ConfigView>,
    ) -> Result<Vec<String>, ApiError> {
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

    /// Creates a new draft and returns the id
    pub async fn create_draft(&self, auth: AuthContext<'_>) -> Result<Id<String>, ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        Ok(self.config_manager.create_draft(auth.auth.user).await.map(|id| Id { id })?)
    }

    /// Creates a new draft for a specific tenant and returns the id
    /// TODO: implement the multitenancy https://siwuerthphoenix.atlassian.net/browse/NEPROD-1232
    pub async fn create_draft_in_tenant(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
    ) -> Result<Id<String>, ApiError> {
        let id = self.config_manager.create_draft(auth.user()).await?;
        Ok(Id { id })
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

    /// Deploy a draft by id and reload the tornado configuration
    /// TODO: implement the multitenancy https://siwuerthphoenix.atlassian.net/browse/NEPROD-1232
    pub async fn deploy_draft_for_tenant(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
        draft_id: &str,
    ) -> Result<MatcherConfig, ApiError> {
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

    /// Deletes a draft by id for a specific tenant
    /// TODO: implement the multitenancy https://siwuerthphoenix.atlassian.net/browse/NEPROD-1232
    pub async fn delete_draft_in_tenant(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
        draft_id: &str,
    ) -> Result<(), ApiError> {
        let draft = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft)?;
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

    /// TODO: implement the multitenancy https://siwuerthphoenix.atlassian.net/browse/NEPROD-1232
    pub async fn draft_take_over_for_tenant(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
        draft_id: &str,
    ) -> Result<(), ApiError> {
        Ok(self.config_manager.draft_take_over(draft_id, auth.user()).await?)
    }

    async fn get_draft_and_check_owner<T: AuthContextTrait>(
        &self,
        auth: &T,
        draft_id: &str,
    ) -> Result<MatcherConfigDraft, ApiError> {
        let draft = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft)?;
        Ok(draft)
    }

    pub async fn create_draft_config_node(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
        draft_id: &str,
        config: MatcherConfig,
    ) -> Result<(), ApiError> {
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        draft.config.create_node_in_path(&auth.path(), config)?;
        Ok(self.config_manager.update_draft(draft_id, auth.user(), &draft.config).await?)
    }

    pub async fn edit_draft_config_node(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
        draft_id: &str,
        config: MatcherConfig,
    ) -> Result<(), ApiError> {
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;

        draft.config.edit_node_in_path(&auth.path(), config)?;
        Ok(self.config_manager.update_draft(draft_id, auth.user(), &draft.config).await?)
    }

    pub async fn import_draft_config_node(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
        draft_id: &str,
        config: MatcherConfig,
    ) -> Result<(), ApiError> {
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        draft.config.import_node_in_path(&auth.path(), config)?;
        Ok(self.config_manager.update_draft(draft_id, auth.user(), &draft.config).await?)
    }

    pub async fn delete_draft_config_node(
        &self,
        auth: &AuthorizedPath<ConfigEdit>,
        draft_id: &str,
    ) -> Result<(), ApiError> {
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        draft.config.delete_node_in_path(&auth.path())?;
        Ok(self.config_manager.update_draft(draft_id, auth.user(), &draft.config).await?)
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
    use tornado_engine_api_dto::config::{ConstraintDto, RuleDetailsDto};
    use tornado_engine_matcher::config::filter::Filter;
    use tornado_engine_matcher::config::rule::{Constraint, Rule};
    use tornado_engine_matcher::config::{
        Defaultable, MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData,
    };
    use tornado_engine_matcher::error::MatcherError;

    const DRAFT_OWNER_ID: &str = "OWNER";

    struct TestConfigManager {}

    #[async_trait::async_trait(? Send)]
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

    #[async_trait::async_trait(? Send)]
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

    #[async_trait(? Send)]
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
                MatcherConfig::Ruleset { name: "n".to_owned(), rules: vec![] },
            )
            .await
            .is_err());
        assert!(api
            .update_draft(
                owner_view,
                "id",
                MatcherConfig::Ruleset { name: "n".to_owned(), rules: vec![] },
            )
            .await
            .is_err());
        assert!(api
            .update_draft(
                owner_edit,
                "id",
                MatcherConfig::Ruleset { name: "n".to_owned(), rules: vec![] },
            )
            .await
            .is_ok());
        assert!(api
            .update_draft(
                owner_edit_and_view,
                "id",
                MatcherConfig::Ruleset { name: "n".to_owned(), rules: vec![] },
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
    async fn get_current_config_processing_tree_nodes_by_empty_path_should_return_error_if_authorized_path_does_not_exist(
    ) {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let user_root_3 = AuthorizedPath::new(
            DRAFT_OWNER_ID.to_owned(),
            vec!["root".to_owned(), "root_3".to_owned()],
        );

        // Act
        let res = api.get_current_config_processing_tree_nodes_by_path(&user_root_3).await;

        // Assert
        assert!(res.is_err());
    }

    #[actix_rt::test]
    async fn get_current_config_node_details_by_path_should_return_details_starting_from_authorized_path(
    ) {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let user = AuthorizedPath::new(
            DRAFT_OWNER_ID.to_owned(),
            vec!["root".to_owned(), "root_1".to_owned(), "root_1_2".to_owned()],
        );

        let config = &api.config_manager.get_config().await.unwrap();

        // Act
        let res_get_node_details = api.get_node_details(&user, config).await.unwrap();
        let res = api.get_current_config_node_details_by_path(&user).await.unwrap();

        // Assert
        let expected_res = ProcessingTreeNodeDetailsDto::Ruleset {
            name: "root_1_2".to_string(),
            rules: vec![RuleDetailsDto {
                name: "root_1_2_1".to_string(),
                description: "".to_string(),
                do_continue: false,
                active: true,
                actions: vec![],
            }],
        };
        assert_eq!(res, expected_res);
        assert_eq!(res_get_node_details, expected_res);
    }

    #[actix_rt::test]
    async fn get_current_config_rule_by_path_should_return_dto() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let user = AuthorizedPath::new(
            DRAFT_OWNER_ID.to_owned(),
            vec!["root".to_owned(), "root_1".to_owned(), "root_1_2".to_owned()],
        );

        let config = &api.config_manager.get_config().await.unwrap();

        // Act
        let res_get_rule_details = api.get_rule_details(&user, config, "root_1_2_1").await.unwrap();

        // Assert
        let expected_res = RuleDto {
            name: "root_1_2_1".to_string(),
            description: "".to_string(),
            do_continue: false,
            active: true,
            constraint: ConstraintDto { where_operator: None, with: Default::default() },
            actions: vec![],
        };
        assert_eq!(res_get_rule_details, expected_res);
    }

    #[actix_rt::test]
    async fn get_tree_info_should_return_aggregate_number_of_filters_and_rules() {
        // Arrange
        let test = TestConfigManager {}.get_config().await.unwrap();
        let root = test.get_node_by_path(&["root"]).unwrap();

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(root);

        // Assert
        let expected = TreeInfoDto { rules_count: 2, filters_count: 4 };

        assert_eq!(result, expected);
    }

    #[test]
    fn get_tree_info_should_work_on_filter_without_children() {
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

        let root = test.get_node_by_path(&["root"]).unwrap();

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(root);

        // Assert
        let expected = TreeInfoDto { rules_count: 0, filters_count: 1 };

        assert_eq!(result, expected);
    }

    #[test]
    fn get_tree_info_should_work_on_ruleset_as_only_node() {
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

        let root = test.get_node_by_path(&["root"]).unwrap();

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(root);

        // Assert
        let expected = TreeInfoDto { rules_count: 1, filters_count: 0 };

        assert_eq!(result, expected);
    }

    #[test]
    fn get_tree_info_should_return_zero_rules_on_empty_ruleset() {
        // Arrange
        let test = MatcherConfig::Ruleset { name: "root".to_owned(), rules: vec![] };

        let root = test.get_node_by_path(&["root"]).unwrap();

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(root);

        // Assert
        let expected = TreeInfoDto { rules_count: 0, filters_count: 0 };

        assert_eq!(result, expected);
    }

    #[actix_rt::test]
    async fn get_authorized_tree_info_should_require_permissions() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let user1: AuthorizedPath<ConfigView> =
            AuthorizedPath::new(DRAFT_OWNER_ID.to_owned(), vec!["root".to_owned()]);

        // Act
        let res1 = api.get_authorized_tree_info(&user1).await;

        // Assert
        assert!(res1.is_ok());
    }

    #[actix_rt::test]
    async fn get_authorized_tree_info_should_include_root_node() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let user1 = AuthorizedPath::new(DRAFT_OWNER_ID.to_owned(), vec!["root".to_owned()]);

        let user2 = AuthorizedPath::new(
            DRAFT_OWNER_ID.to_owned(),
            vec!["root".to_owned(), "root_1".to_owned(), "root_1_2".to_owned()],
        );

        // Act
        let result1 = api.get_authorized_tree_info(&user1).await.unwrap();
        let result2 = api.get_authorized_tree_info(&user2).await.unwrap();

        // Assert
        let expected1 = TreeInfoDto { rules_count: 2, filters_count: 4 };
        let expected2 = TreeInfoDto { rules_count: 1, filters_count: 0 };

        assert_eq!(expected1, result1);
        assert_eq!(expected2, result2);
    }

    #[actix_rt::test]
    async fn export_draft_tree_starting_from_a_specific_filter() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let user = AuthorizedPath::new(DRAFT_OWNER_ID.to_owned(), vec!["ruleset".to_owned()]);

        // Act
        let result = api.export_draft_tree_starting_from_node_path(&user, "id").await.unwrap();

        // Assert
        let expected = MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![] };
        assert_eq!(expected, result);
    }
}
