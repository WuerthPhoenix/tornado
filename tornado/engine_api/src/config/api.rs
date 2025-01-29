use crate::auth::auth_v2::AuthContextV2;
use crate::auth::{AuthContext, AuthContextTrait, Permission};
use crate::config::convert::{dto_into_rule, rule_into_dto};
use crate::error::ApiError;
use log::*;
use std::sync::Arc;
use tornado_engine_api_dto::common::Id;
use tornado_engine_api_dto::config::{
    ProcessingTreeNodeConfigDto, ProcessingTreeNodeDetailsDto, RuleDto, TreeInfoDto,
};
use tornado_engine_matcher::config::operation::{matcher_config_filter, NodeFilter};
use tornado_engine_matcher::config::{
    MatcherConfig, MatcherConfigDraft, MatcherConfigEditor, MatcherConfigReader,
};

const NODE_PATH_SEPARATOR: &str = ",";

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
#[async_trait::async_trait(? Send)]
pub trait ConfigApiHandler: Send + Sync {
    async fn reload_configuration(&self) -> Result<MatcherConfig, ApiError>;
}

pub struct ConfigApi<A: ConfigApiHandler, CM: MatcherConfigReader + MatcherConfigEditor + ?Sized> {
    handler: A,
    config_manager: Arc<CM>,
}

impl<A: ConfigApiHandler, CM: MatcherConfigReader + MatcherConfigEditor + ?Sized> ConfigApi<A, CM> {
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
        node_path: Option<&str>,
    ) -> Result<Vec<ProcessingTreeNodeConfigDto>, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        let relative_node_path: Vec<_> = node_path
            .map(|node_path| node_path.split(NODE_PATH_SEPARATOR).collect())
            .unwrap_or_default();

        let filtered_matcher =
            get_filtered_matcher(&self.config_manager.get_config().await?, &auth).await?;
        self.get_authorized_child_nodes(&auth, relative_node_path, filtered_matcher).await
    }

    async fn get_authorized_child_nodes(
        &self,
        auth: &AuthContextV2<'_>,
        relative_node_path: Vec<&str>,
        filtered_matcher: MatcherConfig,
    ) -> Result<Vec<ProcessingTreeNodeConfigDto>, ApiError> {
        let authorized_path =
            auth.auth.authorization.path.iter().map(|s| s as &str).collect::<Vec<_>>();

        // We must remove the last element of the authorized path because node_path starts from
        // the entry point (included) of the authorized tree.
        // It is safe to pop from the authorized path because the MatcherConfig is already filtered.
        let absolute_node_path = pop_authorized_path_and_append_relative_path(
            authorized_path,
            relative_node_path.clone(),
        )?;

        let child_nodes = filtered_matcher
            .get_child_nodes_by_path(absolute_node_path.as_slice())
            .ok_or(ApiError::NodeNotFoundError {
            message: format!("Node for relative path {:?} not found", relative_node_path),
        })?;
        let has_iterator_ancestor =
            filtered_matcher.has_iterator_in_path(absolute_node_path.as_slice());
        Ok(child_nodes
            .iter()
            .map(|node| ProcessingTreeNodeConfigDto::convert(node, has_iterator_ancestor))
            .collect())
    }

    pub async fn get_authorized_tree_info(
        &self,
        auth: &AuthContextV2<'_>,
    ) -> Result<TreeInfoDto, ApiError> {
        auth.has_any_permission(&[&Permission::ConfigView, &Permission::ConfigEdit])?;

        let filtered_matcher =
            get_filtered_matcher(&self.config_manager.get_config().await?, auth).await?;

        let mut absolute_path: Vec<_> =
            auth.auth.authorization.path.iter().map(|s| s as &str).collect();

        // We must remove the last element of the authorized path because the endpoint is expected
        // to return the entry point of the authorized tree, when node_path is empty.
        // It is safe to pop from the authorized path because the MatcherConfig is already filtered.
        if absolute_path.pop().is_none() {
            let message = "The authorized node path cannot be empty.";
            warn!("{}", message);
            return Err(ApiError::InvalidAuthorizedPath { message: message.to_owned() });
        }

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
                    TreeInfoDto { filters_count: 1, ..Default::default() }
                        + Self::fetch_tree_info(nodes)
                }
                MatcherConfig::Ruleset { rules, .. } => {
                    TreeInfoDto { rules_count: rules.len(), ..Default::default() }
                }
                MatcherConfig::Iterator { nodes, .. } => {
                    TreeInfoDto { iterators_count: 1, ..Default::default() }
                        + Self::fetch_tree_info(nodes)
                }
            })
            .sum()
    }

    pub async fn get_current_config_node_details_by_path(
        &self,
        auth: AuthContextV2<'_>,
        node_path: &str,
    ) -> Result<ProcessingTreeNodeDetailsDto, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        let filtered_matcher =
            get_filtered_matcher(&self.config_manager.get_config().await?, &auth).await?;
        self.get_node_details(&auth, &filtered_matcher, node_path).await
    }

    pub async fn export_draft_tree_starting_from_node_path(
        &self,
        auth: AuthContextV2<'_>,
        draft_id: &str,
        relative_node_path: &str,
    ) -> Result<MatcherConfig, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        let draft_config = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft_config)?;
        let absolute_node_path = self.get_absolute_path_from_relative(&auth, relative_node_path)?;
        let filtered_matcher = get_filtered_matcher(&draft_config.config, &auth).await?;
        let node = filtered_matcher.get_node_by_path(absolute_node_path.as_slice()).ok_or(
            ApiError::NodeNotFoundError {
                message: format!("Node for relative path {:?} not found", relative_node_path),
            },
        )?;
        Ok(node.to_owned())
    }

    pub async fn get_draft_config_node_details_by_path(
        &self,
        auth: AuthContextV2<'_>,
        draft_id: &str,
        node_path: &str,
    ) -> Result<ProcessingTreeNodeDetailsDto, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        let draft_config = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft_config)?;
        let filtered_matcher = get_filtered_matcher(&draft_config.config, &auth).await?;
        self.get_node_details(&auth, &filtered_matcher, node_path).await
    }

    async fn get_node_details(
        &self,
        auth: &AuthContextV2<'_>,
        filtered_matcher: &MatcherConfig,
        relative_node_path: &str,
    ) -> Result<ProcessingTreeNodeDetailsDto, ApiError> {
        let absolute_node_path = self.get_absolute_path_from_relative(auth, relative_node_path)?;

        let node = filtered_matcher.get_node_by_path(absolute_node_path.as_slice()).ok_or(
            ApiError::NodeNotFoundError {
                message: format!("Node for relative path {:?} not found", relative_node_path),
            },
        )?;
        Ok(ProcessingTreeNodeDetailsDto::from(node))
    }

    fn get_absolute_path_from_relative<'a>(
        &self,
        auth: &'a AuthContextV2,
        relative_node_path: &'a str,
    ) -> Result<Vec<&'a str>, ApiError> {
        let authorized_path =
            auth.auth.authorization.path.iter().map(|s| s as &str).collect::<Vec<_>>();

        let relative_node_path = relative_node_path.split(NODE_PATH_SEPARATOR).collect::<Vec<_>>();

        if authorized_path.last() != relative_node_path.first() {
            return Err(self.get_unauthorized_path_error());
        }
        // We must remove the last element of the authorized path because node_path starts from
        // the entry point (included) of the authorized tree.
        // It is safe to pop from the authorized path because the MatcherConfig is already filtered.
        let absolute_node_path =
            pop_authorized_path_and_append_relative_path(authorized_path, relative_node_path)?;
        Ok(absolute_node_path)
    }

    /// Returns processing tree node details by path
    /// in the current configuration of tornado
    pub async fn get_current_rule_details_by_path(
        &self,
        auth: &AuthContextV2<'_>,
        ruleset_path: &str,
        rule_name: &str,
    ) -> Result<RuleDto, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        let filtered_matcher =
            get_filtered_matcher(&self.config_manager.get_config().await?, auth).await?;
        self.get_rule_details(auth, &filtered_matcher, ruleset_path, rule_name).await
    }

    /// Returns processing tree node details by path
    /// in the current configuration of tornado
    pub async fn get_draft_rule_details_by_path(
        &self,
        auth: &AuthContextV2<'_>,
        draft_id: &str,
        ruleset_path: &str,
        rule_name: &str,
    ) -> Result<RuleDto, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        let draft_config = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft_config)?;
        let filtered_matcher = get_filtered_matcher(&draft_config.config, auth).await?;
        self.get_rule_details(auth, &filtered_matcher, ruleset_path, rule_name).await
    }

    pub async fn create_draft_rule_details_by_path(
        &self,
        auth: AuthContextV2<'_>,
        draft_id: &str,
        ruleset_path: &str,
        rule_dto: RuleDto,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        let absolute_node_path = self.get_absolute_path_from_relative(&auth, ruleset_path)?;
        let rule = dto_into_rule(rule_dto)?;
        draft.config.create_rule(&absolute_node_path, rule)?;
        Ok(self
            .config_manager
            .update_draft(draft_id, auth.auth.user.clone(), &draft.config)
            .await?)
    }

    pub async fn edit_draft_rule_details_by_path(
        &self,
        auth: AuthContextV2<'_>,
        draft_id: &str,
        ruleset_path: &str,
        rule_name: &str,
        rule_dto: RuleDto,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        let absolute_node_path = self.get_absolute_path_from_relative(&auth, ruleset_path)?;
        let rule = dto_into_rule(rule_dto)?;
        draft.config.edit_rule(&absolute_node_path, rule_name, rule)?;
        Ok(self
            .config_manager
            .update_draft(draft_id, auth.auth.user.clone(), &draft.config)
            .await?)
    }

    pub async fn move_draft_rule_by_path(
        &self,
        auth: AuthContextV2<'_>,
        draft_id: &str,
        ruleset_path: &str,
        rule_name: &str,
        position: usize,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        let absolute_node_path = self.get_absolute_path_from_relative(&auth, ruleset_path)?;
        draft.config.move_rule(&absolute_node_path, rule_name, position)?;
        Ok(self
            .config_manager
            .update_draft(draft_id, auth.auth.user.clone(), &draft.config)
            .await?)
    }

    pub async fn delete_draft_rule_details_by_path(
        &self,
        auth: AuthContextV2<'_>,
        draft_id: &str,
        ruleset_path: &str,
        rule_name: &str,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        let absolute_node_path = self.get_absolute_path_from_relative(&auth, ruleset_path)?;
        draft.config.delete_rule(&absolute_node_path, rule_name)?;
        Ok(self
            .config_manager
            .update_draft(draft_id, auth.auth.user.clone(), &draft.config)
            .await?)
    }

    async fn get_rule_details(
        &self,
        auth: &AuthContextV2<'_>,
        filtered_matcher: &MatcherConfig,
        ruleset_path: &str,
        rule_name: &str,
    ) -> Result<RuleDto, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        let absolute_node_path = self.get_absolute_path_from_relative(auth, ruleset_path)?;

        let node = filtered_matcher.get_node_by_path(absolute_node_path.as_slice()).ok_or(
            ApiError::NodeNotFoundError {
                message: format!("Node for relative path {:?} not found", ruleset_path),
            },
        )?;

        match node {
            MatcherConfig::Filter { .. } => Err(ApiError::NodeNotFoundError {
                message: format!("Found filter instead of ruleset. Path: {:?}", ruleset_path),
            }),
            MatcherConfig::Iterator { .. } => Err(ApiError::NodeNotFoundError {
                message: format!("Found iterator instead of ruleset. Path: {:?}", ruleset_path),
            }),
            MatcherConfig::Ruleset { name: _, rules } => {
                let rule = rules.iter().find(|rule| rule.name == rule_name);
                if let Some(rule) = rule.cloned() {
                    rule_into_dto(rule).map_err(|err| ApiError::InternalServerError {
                        cause: format!("Couldn't convert rule into dto. Error: {}", err),
                    })
                } else {
                    Err(ApiError::NodeNotFoundError {
                        message: format!(
                            "Couldn't find the rule {} in the {:?} ruleset.",
                            rule_name, ruleset_path
                        ),
                    })
                }
            }
        }
    }

    fn get_unauthorized_path_error(&self) -> ApiError {
        ApiError::ForbiddenError {
            code: "403".to_string(),
            message: "Cannot access node outside authorized path".to_string(),
            params: Default::default(),
        }
    }

    /// Returns child processing tree nodes of a node found by a path
    /// of the draft configuration of tornado
    pub async fn get_draft_config_processing_tree_nodes_by_path(
        &self,
        auth: AuthContextV2<'_>,
        draft_id: &str,
        node_path: Option<&str>,
    ) -> Result<Vec<ProcessingTreeNodeConfigDto>, ApiError> {
        auth.has_permission(&Permission::ConfigView)?;
        let relative_node_path: Vec<_> = node_path
            .map(|node_path| node_path.split(NODE_PATH_SEPARATOR).collect())
            .unwrap_or_default();

        let draft_config = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft_config)?;
        let filtered_matcher = get_filtered_matcher(&draft_config.config, &auth).await?;
        self.get_authorized_child_nodes(&auth, relative_node_path, filtered_matcher).await
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
        auth: &AuthContextV2<'_>,
    ) -> Result<Vec<String>, ApiError> {
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

    /// Creates a new draft and returns the id
    pub async fn create_draft(&self, auth: AuthContext<'_>) -> Result<Id<String>, ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        Ok(self.config_manager.create_draft(auth.auth.user).await.map(|id| Id { id })?)
    }

    /// Creates a new draft for a specific tenant and returns the id
    /// TODO: implement the multitenancy https://siwuerthphoenix.atlassian.net/browse/NEPROD-1232
    pub async fn create_draft_in_tenant(
        &self,
        auth: &AuthContextV2<'_>,
    ) -> Result<Id<String>, ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        Ok(self.config_manager.create_draft(auth.clone().auth.user).await.map(|id| Id { id })?)
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
        auth: &AuthContextV2<'_>,
        draft_id: &str,
    ) -> Result<MatcherConfig, ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        let draft = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft)?;
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
        auth: &AuthContextV2<'_>,
        draft_id: &str,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
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
        auth: &AuthContextV2<'_>,
        draft_id: &str,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        Ok(self.config_manager.draft_take_over(draft_id, auth.clone().auth.user).await?)
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
        auth: AuthContextV2<'_>,
        draft_id: &str,
        node_path: &str,
        config: MatcherConfig,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        let absolute_node_path = self.get_absolute_path_from_relative(&auth, node_path)?;

        draft.config.create_node_in_path(&absolute_node_path, config)?;
        Ok(self.config_manager.update_draft(draft_id, auth.auth.user, &draft.config).await?)
    }

    pub async fn edit_draft_config_node(
        &self,
        auth: AuthContextV2<'_>,
        draft_id: &str,
        node_path: &str,
        config: MatcherConfig,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        let absolute_node_path = self.get_absolute_path_from_relative(&auth, node_path)?;

        draft.config.edit_node_in_path(&absolute_node_path, config)?;
        Ok(self.config_manager.update_draft(draft_id, auth.auth.user, &draft.config).await?)
    }

    pub async fn import_draft_config_node(
        &self,
        auth: AuthContextV2<'_>,
        draft_id: &str,
        node_path: &str,
        config: MatcherConfig,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        let absolute_node_path = self.get_absolute_path_from_relative(&auth, node_path)?;

        draft.config.replace_node(&absolute_node_path, config)?;
        Ok(self.config_manager.update_draft(draft_id, auth.auth.user, &draft.config).await?)
    }

    pub async fn delete_draft_config_node(
        &self,
        auth: AuthContextV2<'_>,
        draft_id: &str,
        node_path: &str,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        let mut draft = self.get_draft_and_check_owner(&auth, draft_id).await?;
        let absolute_node_path = self.get_absolute_path_from_relative(&auth, node_path)?;
        draft.config.delete_node_in_path(&absolute_node_path)?;
        Ok(self.config_manager.update_draft(draft_id, auth.auth.user, &draft.config).await?)
    }
}

pub async fn get_filtered_matcher(
    config: &MatcherConfig,
    auth: &AuthContextV2<'_>,
) -> Result<MatcherConfig, ApiError> {
    let node_filter = NodeFilter::map_from(&[auth.auth.authorization.path.clone()]);
    matcher_config_filter(config, &node_filter).ok_or_else(|| {
        let message = "The authorized node path does not exist.";
        warn!("{} Path: {:?}", message, &auth.auth.authorization.path);
        ApiError::InvalidAuthorizedPath { message: message.to_owned() }
    })
}

fn pop_authorized_path_and_append_relative_path<'a>(
    mut base_path: Vec<&'a str>,
    mut relative_path: Vec<&'a str>,
) -> Result<Vec<&'a str>, ApiError> {
    match base_path.pop() {
        None => {
            let message = "The authorized node path cannot be empty.";
            warn!("ConfigApi - {}", message);
            Err(ApiError::InvalidAuthorizedPath { message: message.to_owned() })
        }
        Some(last_node_base_path) => match relative_path.first() {
            Some(first_node_relative_path) if first_node_relative_path != &last_node_base_path => {
                Err(ApiError::BadRequestError {
                    cause: "Node path does not comply with authorized path".to_string(),
                })
            }
            _ => {
                base_path.append(&mut relative_path);
                Ok(base_path)
            }
        },
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
    use tornado_engine_api_dto::config::{ConstraintDto, RuleDetailsDto};
    use tornado_engine_matcher::config::nodes::Filter;
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
            has_iterator_ancestor: false,
            active: false,
        }];
        assert_eq!(
            api.get_current_config_processing_tree_nodes_by_path(not_owner_edit_and_view, None)
                .await
                .unwrap(),
            expected_result
        );
        assert_eq!(
            api.get_current_config_processing_tree_nodes_by_path(owner_view, None).await.unwrap(),
            expected_result
        );
        assert!(api
            .get_current_config_processing_tree_nodes_by_path(owner_edit, None)
            .await
            .is_err());
        assert_eq!(
            api.get_current_config_processing_tree_nodes_by_path(owner_edit_and_view, None)
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
        let config = &api.config_manager.get_config().await.unwrap();
        let filtered_matcher = get_filtered_matcher(config, &user_root_1).await.unwrap();

        // Act
        let res_authorized_child_nodes =
            api.get_authorized_child_nodes(&user_root_1, vec![], filtered_matcher).await.unwrap();
        let res =
            api.get_current_config_processing_tree_nodes_by_path(user_root_1, None).await.unwrap();

        // Assert
        let expected_result = vec![ProcessingTreeNodeConfigDto::Filter {
            name: "root_1".to_string(),
            rules_count: 1,
            children_count: 2,
            description: "".to_string(),
            has_iterator_ancestor: false,
            active: false,
        }];
        assert_eq!(res, expected_result);
        assert_eq!(res_authorized_child_nodes, expected_result);
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
        let config = &api.config_manager.get_config().await.unwrap();

        // Act
        let filtered_matcher = get_filtered_matcher(config, &user_root_3).await;
        let res = api.get_current_config_processing_tree_nodes_by_path(user_root_3, None).await;

        // Assert
        assert!(filtered_matcher.is_err());
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
        let config = &api.config_manager.get_config().await.unwrap();

        // Act
        let filtered_matcher = get_filtered_matcher(config, &user).await;
        let res = api.get_current_config_processing_tree_nodes_by_path(user, None).await;

        // Assert
        assert!(filtered_matcher.is_err());
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
        let config = &api.config_manager.get_config().await.unwrap();

        // Act
        let filtered_matcher = get_filtered_matcher(config, &user).await;

        // Assert
        assert!(filtered_matcher.is_err());
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
        let config = &api.config_manager.get_config().await.unwrap();
        let filtered_matcher = get_filtered_matcher(config, &user_root_1).await.unwrap();

        // Act
        let res_authorized_child_nodes = api
            .get_authorized_child_nodes(&user_root_1, vec!["root_1"], filtered_matcher)
            .await
            .unwrap();
        let res = api
            .get_current_config_processing_tree_nodes_by_path(user_root_1, Some("root_1"))
            .await
            .unwrap();

        // Assert
        let expected = vec![
            ProcessingTreeNodeConfigDto::Filter {
                name: "root_1_1".to_string(),
                rules_count: 0,
                children_count: 0,
                description: "".to_string(),
                active: false,
                has_iterator_ancestor: false,
            },
            ProcessingTreeNodeConfigDto::Ruleset { name: "root_1_2".to_string(), rules_count: 1 },
        ];
        assert_eq!(res, expected);
        assert_eq!(res_authorized_child_nodes, expected);
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
        let config = &api.config_manager.get_config().await.unwrap();

        // Act
        let filtered_matcher = get_filtered_matcher(config, &user_root_3).await;

        // Assert
        assert!(filtered_matcher.is_err());
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
            api.get_current_config_node_details_by_path(not_owner_edit_and_view, "root").await,
            Err(ApiError::ForbiddenError { .. })
        ));
        assert!(!matches!(
            api.get_current_config_node_details_by_path(owner_view, "root").await,
            Err(ApiError::ForbiddenError { .. })
        ));
        assert!(matches!(
            api.get_current_config_node_details_by_path(owner_edit, "root").await,
            Err(ApiError::ForbiddenError { .. })
        ));
        assert!(!matches!(
            api.get_current_config_node_details_by_path(owner_edit_and_view, "root").await,
            Err(ApiError::ForbiddenError { .. })
        ));
    }

    #[actix_rt::test]
    async fn get_current_config_node_details_by_path_should_return_details_starting_from_authorized_path(
    ) {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let user = AuthContextV2::new(
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
        let config = &api.config_manager.get_config().await.unwrap();
        let filtered_matcher = get_filtered_matcher(config, &user).await.unwrap();

        // Act
        let res_get_node_details =
            api.get_node_details(&user, &filtered_matcher, "root_1,root_1_2").await.unwrap();
        let res =
            api.get_current_config_node_details_by_path(user, "root_1,root_1_2").await.unwrap();

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
        let permissions_map = auth_permissions();
        let user = AuthContextV2::new(
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
        let config = &api.config_manager.get_config().await.unwrap();
        let filtered_matcher = get_filtered_matcher(config, &user).await.unwrap();

        // Act
        let res_get_rule_details = api
            .get_rule_details(&user, &filtered_matcher, "root_1,root_1_2", "root_1_2_1")
            .await
            .unwrap();

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
    async fn get_current_config_rule_by_path_should_return_forbidden_if_user_is_not_viewer() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let user = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned(), "root_1".to_owned()],
                    roles: vec!["edit".to_owned()],
                },
                preferences: None,
            },
            &permissions_map,
        );
        let config = &api.config_manager.get_config().await.unwrap();
        let filtered_matcher = get_filtered_matcher(config, &user).await.unwrap();

        // Act
        let res_get_rule_details_error =
            api.get_rule_details(&user, &filtered_matcher, "root_1,root_1_2", "root_1_2_1").await;

        // Assert
        let expected_res = Err(ApiError::ForbiddenError {
            code: "MISSING_REQUIRED_PERMISSIONS".to_string(),
            message: "User [OWNER] does not have the required permissions [[ConfigView]]"
                .to_string(),
            params: Default::default(),
        });
        assert_eq!(expected_res, res_get_rule_details_error);
    }

    #[actix_rt::test]
    async fn get_current_config_node_details_by_path_should_return_error_if_authorized_path_does_not_exist(
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
        let config = &api.config_manager.get_config().await.unwrap();
        let filtered_matcher = get_filtered_matcher(config, &user_root_3).await;

        // Act & Assert
        assert!(filtered_matcher.is_err());
    }

    #[actix_rt::test]
    async fn get_current_config_node_details_by_path_should_return_error_if_node_path_outside_authorized_path(
    ) {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let user = AuthContextV2::new(
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
        let config = &api.config_manager.get_config().await.unwrap();
        let filtered_matcher = get_filtered_matcher(config, &user).await.unwrap();

        // Act & Assert
        assert!(matches!(
            api.get_node_details(&user, &filtered_matcher, "root,root_2").await,
            Err(ApiError::ForbiddenError { .. })
        ))
    }

    #[test]
    fn pop_authorized_path_and_append_relative_path_should_pop_and_append() {
        // Arrange
        let base_path = vec!["root"];
        let relative_path = vec!["root", "child_1"];

        // Act
        let result =
            pop_authorized_path_and_append_relative_path(base_path, relative_path).unwrap();

        // Assert
        assert_eq!(result, vec!["root", "child_1"]);
    }

    #[test]
    fn pop_authorized_path_and_append_relative_path_should_return_error_for_empty_authorized_path()
    {
        // Arrange
        let base_path = vec![];
        let relative_path = vec!["root", "child_1"];

        // Act
        let result = pop_authorized_path_and_append_relative_path(base_path, relative_path);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn pop_authorized_path_and_append_relative_path_should_return_error_if_relative_path_starts_with_element_different_than_last_element_of_authorized_path(
    ) {
        // Arrange
        let base_path = vec!["root"];
        let relative_path = vec!["root_wrong", "child_1"];

        // Act
        let result = pop_authorized_path_and_append_relative_path(base_path, relative_path);

        // Assert
        assert!(matches!(result, Err(ApiError::BadRequestError { .. })));
    }

    #[actix_rt::test]
    async fn get_tree_info_should_return_aggregate_number_of_filters_and_rules() {
        // Arrange
        let test = TestConfigManager {}.get_config().await.unwrap();
        let root = test.get_child_nodes_by_path(&[]).unwrap();

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(&root);

        // Assert
        let expected = TreeInfoDto { rules_count: 2, filters_count: 4, iterators_count: 0 };

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

        let root = test.get_child_nodes_by_path(&[]).unwrap();

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(&root);

        // Assert
        let expected = TreeInfoDto { filters_count: 1, ..Default::default() };

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

        let root = test.get_child_nodes_by_path(&[]).unwrap();

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(&root);

        // Assert
        let expected = TreeInfoDto { rules_count: 1, ..Default::default() };

        assert_eq!(result, expected);
    }

    #[test]
    fn get_tree_info_should_return_zero_rules_on_empty_ruleset() {
        // Arrange
        let test = MatcherConfig::Ruleset { name: "root".to_owned(), rules: vec![] };

        let root = test.get_child_nodes_by_path(&[]).unwrap();

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(&root);

        // Assert
        let expected = TreeInfoDto::default();

        assert_eq!(result, expected);
    }

    #[test]
    fn get_tree_info_should_work_on_empty_config() {
        // Arrange
        let root = [];

        // Act
        let result = ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(&root);

        // Assert
        let expected = Default::default();

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

    #[actix_rt::test]
    async fn get_authorized_tree_info_should_include_root_node() {
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
                authorization: Authorization {
                    path: vec!["root".to_owned(), "root_1".to_owned(), "root_1_2".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            &permissions_map,
        );

        // Act
        let result1 = api.get_authorized_tree_info(&user1).await.unwrap();
        let result2 = api.get_authorized_tree_info(&user2).await.unwrap();

        // Assert
        let expected1 = TreeInfoDto { rules_count: 2, filters_count: 4, iterators_count: 0 };
        let expected2 = TreeInfoDto { rules_count: 1, ..Default::default() };

        assert_eq!(expected1, result1);
        assert_eq!(expected2, result2);
    }

    #[actix_rt::test]
    async fn should_count_iterator_nodes_in_test_config() {
        let test_config = MatcherConfig::Filter {
            name: "".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Ruleset {
                    name: "".to_string(),
                    rules: vec![Rule::default(), Rule::default()],
                },
                MatcherConfig::Filter {
                    name: "".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Iterator {
                        name: "".to_string(),
                        iterator: Default::default(),
                        nodes: vec![],
                    }],
                },
                MatcherConfig::Iterator {
                    name: "".to_string(),
                    iterator: Default::default(),
                    nodes: vec![
                        MatcherConfig::Ruleset {
                            name: "".to_string(),
                            rules: vec![Rule::default()],
                        },
                        MatcherConfig::Filter {
                            name: "".to_string(),
                            filter: Default::default(),
                            nodes: vec![],
                        },
                    ],
                },
            ],
        };

        let expected = TreeInfoDto { rules_count: 3, filters_count: 3, iterators_count: 2 };

        let result =
            ConfigApi::<TestApiHandler, TestConfigManager>::fetch_tree_info(&[test_config]);

        assert_eq!(expected, result);
    }

    #[actix_rt::test]
    async fn export_draft_tree_starting_from_a_specific_filter() {
        // Arrange
        let api = ConfigApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();
        let user = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["ruleset".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            &permissions_map,
        );

        // Act
        let result = api
            .export_draft_tree_starting_from_node_path(user, "id", "ruleset".as_ref())
            .await
            .unwrap();

        // Assert
        let expected = MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![] };
        assert_eq!(expected, result);
    }
}
