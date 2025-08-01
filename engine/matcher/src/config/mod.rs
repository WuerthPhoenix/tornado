use crate::config::nodes::{Filter, MatcherIterator};
use crate::config::rule::Rule;
use crate::config::v2::{ConfigNodeDir, ConfigType};
use crate::error::MatcherError;
use crate::matcher;
use crate::matcher::Matcher;
use serde::{de::Deserializer, Deserialize, Serialize};
use std::borrow::Cow;

pub mod nodes;
pub mod operation;
pub mod rule;
pub mod v1;
pub mod v2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MatcherConfigDraft {
    pub data: MatcherConfigDraftData,
    pub config: MatcherConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MatcherConfigDraftData {
    pub created_ts_ms: i64,
    pub updated_ts_ms: i64,
    pub user: String,
    pub draft_id: String,
}

impl ConfigNodeDir for MatcherConfigDraftData {
    fn config_type() -> ConfigType {
        ConfigType::Draft
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum MatcherConfig {
    Filter { name: String, filter: Filter, nodes: Vec<MatcherConfig> },
    Iterator { name: String, iterator: MatcherIterator, nodes: Vec<MatcherConfig> },
    Ruleset { name: String, rules: Vec<Rule> },
}

impl MatcherConfig {
    pub fn get_name(&self) -> &str {
        match self {
            MatcherConfig::Filter { name, .. }
            | MatcherConfig::Iterator { name, .. }
            | MatcherConfig::Ruleset { name, .. } => name,
        }
    }

    pub fn get_name_mut(&mut self) -> &mut String {
        match self {
            MatcherConfig::Filter { name, .. }
            | MatcherConfig::Iterator { name, .. }
            | MatcherConfig::Ruleset { name, .. } => name,
        }
    }

    fn get_child_node_by_name(&self, child_name: &str) -> Option<&MatcherConfig> {
        match self {
            MatcherConfig::Filter { nodes, .. } | MatcherConfig::Iterator { nodes, .. } => {
                nodes.iter().find(|child| child.get_name() == child_name)
            }
            MatcherConfig::Ruleset { .. } => None,
        }
    }

    fn get_mut_child_node_by_name(&mut self, child_name: &str) -> Option<&mut MatcherConfig> {
        match self {
            MatcherConfig::Filter { nodes, .. } | MatcherConfig::Iterator { nodes, .. } => {
                nodes.iter_mut().find(|child| child.get_name() == child_name)
            }
            MatcherConfig::Ruleset { .. } => None,
        }
    }

    pub fn get_node_by_path(&self, path: &[&str]) -> Option<&MatcherConfig> {
        // empty path returns None
        if path.is_empty() {
            return None;
        }
        // first element must be current node
        if path[0] != self.get_name() {
            return None;
        }
        let mut current_node = self;
        // drill down from root
        for &node_name in path[1..].iter() {
            if let Some(new_current_node) = current_node.get_child_node_by_name(node_name) {
                current_node = new_current_node
            } else {
                return None;
            }
        }
        Some(current_node)
    }

    fn get_mut_node_by_path(&mut self, path: &[&str]) -> Option<&mut MatcherConfig> {
        // empty path returns None
        if path.is_empty() {
            return None;
        }
        // first element must be current node
        if path[0] != self.get_name() {
            return None;
        }
        let mut current_node = self;
        // drill down from root
        for &node_name in path[1..].iter() {
            if let Some(new_current_node) = current_node.get_mut_child_node_by_name(node_name) {
                current_node = new_current_node
            } else {
                return None;
            }
        }
        Some(current_node)
    }

    fn get_mut_node_by_path_or_err(
        &mut self,
        path: &[&str],
    ) -> Result<&mut MatcherConfig, MatcherError> {
        self.get_mut_node_by_path(path).ok_or_else(|| MatcherError::ConfigurationError {
            message: format!("Node in this path does not exist: {:?}", path),
        })
    }

    fn get_mut_rules_by_path_or_err(
        &mut self,
        path: &[&str],
    ) -> Result<&mut Vec<Rule>, MatcherError> {
        match self.get_mut_node_by_path_or_err(path)? {
            MatcherConfig::Filter { .. } => Err(MatcherError::ConfigurationError {
                message: "Cannot access rules in filter nodes".to_string(),
            }),
            MatcherConfig::Iterator { .. } => Err(MatcherError::ConfigurationError {
                message: "Cannot access rules in iterator nodes".to_string(),
            }),
            MatcherConfig::Ruleset { rules, .. } => Ok(rules),
        }
    }

    // Returns child nodes of a node found by a path
    // If the path is empty [], the [self] is returned
    pub fn get_child_nodes_by_path(&self, path: &[&str]) -> Option<Cow<Vec<MatcherConfig>>> {
        if path.is_empty() {
            return Some(Cow::Owned(vec![self.to_owned()]));
        }
        match self.get_node_by_path(path) {
            Some(MatcherConfig::Filter { nodes, .. })
            | Some(MatcherConfig::Iterator { nodes, .. }) => Some(Cow::Borrowed(nodes)),
            Some(MatcherConfig::Ruleset { .. }) | None => None,
        }
    }

    pub fn has_iterator_in_path(&self, path: &[&str]) -> bool {
        if path.is_empty() {
            return false;
        }
        // trim root from path.
        self.has_iterator_in_path_inner(&path[1..])
    }

    fn has_iterator_in_path_inner(&self, path: &[&str]) -> bool {
        match path {
            [] => false,
            [node_name, path @ ..] => match self.get_child_node_by_name(node_name) {
                Some(MatcherConfig::Iterator { .. }) => true,
                Some(node) => node.has_iterator_in_path_inner(path),
                None => false,
            },
        }
    }

    pub fn contains_iterator(&self) -> bool {
        match self {
            MatcherConfig::Filter { nodes, .. } => {
                nodes.iter().any(MatcherConfig::contains_iterator)
            }
            MatcherConfig::Iterator { .. } => true,
            MatcherConfig::Ruleset { .. } => false,
        }
    }

    // Returns the total amount of direct children of a node
    pub fn get_direct_child_nodes_count(&self) -> usize {
        match self {
            MatcherConfig::Filter { nodes, .. } | MatcherConfig::Iterator { nodes, .. } => {
                nodes.len()
            }
            MatcherConfig::Ruleset { .. } => 0,
        }
    }

    // Returns the total amount of rules of the node and its children
    pub fn get_all_rules_count(&self) -> usize {
        match self {
            MatcherConfig::Filter { nodes, .. } | MatcherConfig::Iterator { nodes, .. } => {
                nodes.iter().map(MatcherConfig::get_all_rules_count).sum()
            }
            MatcherConfig::Ruleset { rules, .. } => rules.len(),
        }
    }

    // Create a node at a specific path
    pub fn create_node_in_path(
        &mut self,
        path: &[&str],
        node: MatcherConfig,
    ) -> Result<(), MatcherError> {
        if path.is_empty() {
            return Err(MatcherError::ConfigurationError {
                message: "The node path must specify a parent node".to_string(),
            });
        }

        if node.contains_iterator() && self.has_iterator_in_path(path) {
            return Err(MatcherError::NestedIteratorError);
        }

        let current_node = self.get_mut_node_by_path_or_err(path)?;
        if current_node.get_child_node_by_name(node.get_name()).is_some() {
            return Err(MatcherError::NotUniqueNameError { name: node.get_name().to_owned() });
        }

        // Validate input before saving it to the draft.
        let _ = Matcher::build(&node)?;

        match current_node {
            MatcherConfig::Ruleset { .. } => Err(MatcherError::ConfigurationError {
                message: "A ruleset cannot have children nodes".to_string(),
            }),
            MatcherConfig::Filter { nodes, .. } | MatcherConfig::Iterator { nodes, .. } => {
                nodes.push(node.clone());
                Ok(())
            }
        }
    }

    // Create a node at a specific path
    pub fn edit_node_in_path(
        &mut self,
        path: &[&str],
        new_node: MatcherConfig,
    ) -> Result<(), MatcherError> {
        if path.is_empty() {
            return Err(MatcherError::ConfigurationError {
                message: "Empty path is not allowed".to_string(),
            });
        }

        let parent_node = self.get_mut_node_by_path_or_err(&path[..path.len() - 1])?;
        let node_name = path[path.len() - 1];
        if node_name != new_node.get_name()
            && parent_node.get_child_node_by_name(new_node.get_name()).is_some()
        {
            return Err(MatcherError::NotUniqueNameError { name: new_node.get_name().to_owned() });
        }

        // Validate input before saving it to the draft.
        let _ = Matcher::build(&new_node)?;

        let old_node = self.get_mut_node_by_path_or_err(path)?;
        match (old_node, new_node) {
            (
                MatcherConfig::Ruleset { name, .. },
                MatcherConfig::Ruleset { name: new_name, .. },
            ) => {
                *name = new_name;
            }
            (
                MatcherConfig::Filter { name, filter, .. },
                MatcherConfig::Filter { name: new_name, filter: new_filter, .. },
            ) => {
                *name = new_name;
                *filter = new_filter;
            }
            (
                MatcherConfig::Iterator { name, iterator, .. },
                MatcherConfig::Iterator { name: new_name, iterator: new_iterator, .. },
            ) => {
                *name = new_name;
                *iterator = new_iterator;
            }
            _ => {
                return Err(MatcherError::ConfigurationError {
                    message: "Node to edit is not of same type of the new one passed".to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn replace_node(
        &mut self,
        path: &[&str],
        new_node: MatcherConfig,
    ) -> Result<(), MatcherError> {
        let old_node = match path {
            [] => {
                return Err(MatcherError::ConfigurationError {
                    message: "Empty path is not allowed".to_string(),
                })
            }
            [_node] => self,
            [parent @ .., node_name] => {
                let parent_node = self.get_mut_node_by_path_or_err(parent)?;
                if *node_name != new_node.get_name()
                    && parent_node.get_child_node_by_name(new_node.get_name()).is_some()
                {
                    return Err(MatcherError::NotUniqueNameError {
                        name: new_node.get_name().to_owned(),
                    });
                }
                self.get_mut_node_by_path_or_err(path)?
            }
        };
        *old_node = new_node;
        Ok(())
    }

    // Create a node at a specific path
    pub fn delete_node_in_path(&mut self, path: &[&str]) -> Result<(), MatcherError> {
        if path.is_empty() {
            return Err(MatcherError::ConfigurationError {
                message: "Empty path is not allowed".to_string(),
            });
        }

        let path_to_parent = &path[0..path.len() - 1];
        let node_to_delete = path.last().unwrap_or(&"");
        let parent_node = self.get_mut_node_by_path_or_err(path_to_parent)?;

        match parent_node {
            MatcherConfig::Filter { nodes, .. } | MatcherConfig::Iterator { nodes, .. } => {
                let num_nodes_before = nodes.len();
                nodes.retain(|n| n.get_name() != *node_to_delete);
                if nodes.len() == num_nodes_before {
                    return Err(MatcherError::ConfigurationError {
                        message: format!(
                            "A node with name {:?} not found in {:?}",
                            node_to_delete, path_to_parent,
                        ),
                    });
                }
                Ok(())
            }
            MatcherConfig::Ruleset { .. } => Err(MatcherError::ConfigurationError {
                message: "Can't delete a node in a ruleset.".to_string(),
            }),
        }
    }

    // Create a node at a specific path
    pub fn create_rule(&mut self, ruleset_path: &[&str], rule: Rule) -> Result<(), MatcherError> {
        // validate rule before saving to the ruleset
        matcher::validate_rule(&rule)?;
        let rules = self.get_mut_rules_by_path_or_err(ruleset_path)?;

        if rules.iter().any(|Rule { name, .. }| name == &rule.name) {
            return Err(MatcherError::ConfigurationError {
                message: format!(
                    "A rule with name {} already exists in ruleset {:?}",
                    rule.name, ruleset_path,
                ),
            });
        };

        rules.push(rule);
        Ok(())
    }

    pub fn edit_rule(
        &mut self,
        ruleset_path: &[&str],
        rule_name: &str,
        new_rule: Rule,
    ) -> Result<(), MatcherError> {
        // validate rule before saving to the ruleset
        matcher::validate_rule(&new_rule)?;
        let rules = self.get_mut_rules_by_path_or_err(ruleset_path)?;

        match rules.iter_mut().find(|rule| rule.name == rule_name) {
            None => Err(MatcherError::ConfigurationError {
                message: format!(
                    "No rule with name {} exists in ruleset {}",
                    new_rule.name, rule_name
                ),
            }),
            Some(rule) => {
                *rule = new_rule;
                Ok(())
            }
        }
    }

    pub fn move_rule(
        &mut self,
        ruleset_path: &[&str],
        rule_name: &str,
        position: usize,
    ) -> Result<(), MatcherError> {
        let rules = self.get_mut_rules_by_path_or_err(ruleset_path)?;

        if position >= rules.len() {
            return Err(MatcherError::ConfigurationError {
                message: format!(
                    "Rule position {} out of bounds for ruleset with {} rules.",
                    position,
                    rules.len()
                ),
            });
        }

        match rules
            .iter()
            .enumerate()
            .find(|(_, rule)| rule.name == rule_name)
            .map(|(index, _)| index)
        {
            None => Err(MatcherError::ConfigurationError {
                message: format!(
                    "No rule with name {} exists in ruleset {:?}",
                    rule_name, ruleset_path
                ),
            }),
            Some(index) => {
                let rule = rules.remove(index);
                rules.insert(position, rule);
                Ok(())
            }
        }
    }

    pub fn delete_rule(
        &mut self,
        ruleset_path: &[&str],
        rule_name: &str,
    ) -> Result<(), MatcherError> {
        let rules = self.get_mut_rules_by_path_or_err(ruleset_path)?;
        let rule_count = rules.len();
        rules.retain(|rule| rule.name != rule_name);

        if rule_count == rules.len() {
            return Err(MatcherError::ConfigurationError {
                message: format!(
                    "No rule with name {} exists in ruleset {:?}",
                    rule_name, ruleset_path
                ),
            });
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum Defaultable<T: Serialize + Clone> {
    #[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
    Value(T),
    Default {},
}

impl<T: Serialize + Clone> Default for Defaultable<T> {
    fn default() -> Self {
        Self::Default {}
    }
}

impl<T: Serialize + Clone> From<Defaultable<T>> for Option<T> {
    fn from(default: Defaultable<T>) -> Self {
        match default {
            Defaultable::Value(value) => Some(value),
            Defaultable::Default {} => None,
        }
    }
}

impl<T: Serialize + Clone> From<Option<T>> for Defaultable<T> {
    fn from(source: Option<T>) -> Self {
        match source {
            Some(value) => Defaultable::Value(value),
            None => Defaultable::Default {},
        }
    }
}

/// A MatcherConfigReader permits to read and manipulate the Tornado Configuration
/// from a configuration source.
#[async_trait::async_trait(? Send)]
pub trait MatcherConfigReader: Sync + Send {
    async fn get_config(&self) -> Result<MatcherConfig, MatcherError>;
}

/// A MatcherConfigEditor permits to edit Tornado Configuration drafts
#[async_trait::async_trait(? Send)]
pub trait MatcherConfigEditor: MatcherConfigReader + Sync + Send {
    /// Returns the list of available drafts
    async fn get_drafts(&self) -> Result<Vec<String>, MatcherError>;

    /// Returns a draft by id
    async fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError>;

    /// Creates a new draft and returns the id
    async fn create_draft(&self, user: String) -> Result<String, MatcherError>;

    /// Update a draft
    async fn update_draft(
        &self,
        draft_id: &str,
        user: String,
        config: &MatcherConfig,
    ) -> Result<(), MatcherError>;

    /// Deploy a draft by id replacing the current tornado configuration
    async fn deploy_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError>;

    /// Deletes a draft by id
    async fn delete_draft(&self, draft_id: &str) -> Result<(), MatcherError>;

    /// Sets the ownership of a draft to a user
    async fn draft_take_over(&self, draft_id: &str, user: String) -> Result<(), MatcherError>;

    /// Deploys a new configuration overriding the current one
    async fn deploy_config(&self, config: &MatcherConfig) -> Result<MatcherConfig, MatcherError>;
}

pub fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rule::{Constraint, Operator};
    use serde_json::json;

    #[test]
    fn test_get_direct_child_nodes_count() {
        // Arrange
        let config_no_children = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![],
        };

        let config_one_children = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "child_ruleset1".to_string(),
                rules: vec![],
            }],
        };

        let config_more_children = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Ruleset { name: "child_ruleset1".to_string(), rules: vec![] },
                MatcherConfig::Filter {
                    name: "child_filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![
                        MatcherConfig::Ruleset {
                            name: "filter1_child_ruleset1".to_string(),
                            rules: vec![],
                        },
                        MatcherConfig::Ruleset {
                            name: "filter1_child_ruleset2".to_string(),
                            rules: vec![],
                        },
                    ],
                },
            ],
        };

        // Act
        let no_children_result = config_no_children.get_direct_child_nodes_count();
        let one_children_result = config_one_children.get_direct_child_nodes_count();
        let config_more_children = config_more_children.get_direct_child_nodes_count();

        // Assert
        assert_eq!(no_children_result, 0);
        assert_eq!(one_children_result, 1);
        assert_eq!(config_more_children, 2);
    }

    #[test]
    fn test_get_all_rules_count() {
        // Arrange
        let config_no_ruleset = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![],
        };

        let config_no_rules = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "child_ruleset1".to_string(),
                rules: vec![],
            }],
        };

        let config_one_rules = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "child_ruleset1".to_string(),
                rules: vec![Rule {
                    name: "rule1".to_string(),
                    description: "".to_string(),
                    do_continue: false,
                    active: false,
                    constraint: Constraint { where_operator: None, with: Default::default() },
                    actions: vec![],
                }],
            }],
        };

        let config_more_rules = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Ruleset {
                    name: "child_ruleset1".to_string(),
                    rules: vec![
                        Rule { name: "rule1".to_string(), ..Default::default() },
                        Rule { name: "rule2".to_string(), ..Default::default() },
                    ],
                },
                MatcherConfig::Ruleset {
                    name: "child_ruleset2".to_string(),
                    rules: vec![Rule { name: "rule3".to_string(), ..Default::default() }],
                },
                MatcherConfig::Filter {
                    name: "child_filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Ruleset {
                        name: "child_ruleset3".to_string(),
                        rules: vec![
                            Rule { name: "rule4".to_string(), ..Default::default() },
                            Rule { name: "rule5".to_string(), ..Default::default() },
                        ],
                    }],
                },
            ],
        };

        // Act
        let no_ruleset_result = config_no_ruleset.get_all_rules_count();
        let no_rules_result = config_no_rules.get_all_rules_count();
        let one_rules_result = config_one_rules.get_all_rules_count();
        let config_more_rules = config_more_rules.get_all_rules_count();

        // Assert
        assert_eq!(no_ruleset_result, 0);
        assert_eq!(no_rules_result, 0);
        assert_eq!(one_rules_result, 1);
        assert_eq!(config_more_rules, 5);
    }

    #[test]
    fn test_get_filter_node_by_path() {
        // Arrange
        let config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Default::default(),
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![],
                        }],
                    }],
                },
            ],
        };

        // Act
        let empty_path = config.get_child_nodes_by_path(&[]);
        let one_level = config.get_child_nodes_by_path(&["root"]);
        let nested_levels = config.get_child_nodes_by_path(&["root", "filter2"]);
        let nested_levels_path_with_ruleset =
            config.get_child_nodes_by_path(&["root", "filter2", "filter3", "ruleset1"]);
        let not_existing_path = config.get_child_nodes_by_path(&["foo", "bar"]);

        // Assert
        assert_eq!(empty_path.clone().unwrap().len(), 1);
        assert!(
            matches!(empty_path.unwrap().first().unwrap(), MatcherConfig::Filter {name, ..} if name == "root")
        );

        assert_eq!(one_level.clone().unwrap().len(), 2);
        assert!(
            matches!(one_level.clone().unwrap().first().unwrap(), MatcherConfig::Filter {name, ..} if name == "filter1")
        );
        assert!(
            matches!(one_level.unwrap().get(1).unwrap(), MatcherConfig::Filter {name, ..} if name == "filter2")
        );

        assert_eq!(nested_levels.clone().unwrap().len(), 1);
        assert!(
            matches!(nested_levels.unwrap().first().unwrap(), MatcherConfig::Filter {name, ..} if name == "filter3")
        );

        assert_eq!(nested_levels_path_with_ruleset, None);
        assert_eq!(not_existing_path, None);
    }

    #[test]
    fn test_get_node_by_path() {
        // Arrange
        let config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter2".to_string(),
                        filter: Default::default(),
                        nodes: vec![
                            MatcherConfig::Filter {
                                name: "filter1".to_string(),
                                filter: Filter {
                                    description: "Filter at last level".to_string(),
                                    active: false,
                                    filter: Defaultable::Default {},
                                },
                                nodes: vec![],
                            },
                            MatcherConfig::Ruleset { name: "ruleset2".to_string(), rules: vec![] },
                        ],
                    }],
                },
                MatcherConfig::Ruleset { name: "Ruleset1".to_string(), rules: vec![] },
            ],
        };

        // Act
        let result_with_empty_path = config.get_node_by_path(&[]);
        let result_with_wrong_path = config.get_node_by_path(&["root", "foo"]);
        let result_with_first_level_path = config.get_node_by_path(&["root"]);
        let result_with_filter_same_name =
            config.get_node_by_path(&["root", "filter1", "filter2", "filter1"]);
        let result_with_ruleset =
            config.get_node_by_path(&["root", "filter1", "filter2", "ruleset2"]);

        // Assert
        assert_eq!(result_with_empty_path, None);
        assert_eq!(result_with_wrong_path, None);
        assert!(
            matches!(result_with_first_level_path.unwrap(), MatcherConfig::Filter {name, ..} if name == "root")
        );
        assert!(
            matches!(result_with_filter_same_name.unwrap(), MatcherConfig::Filter {name, filter, ..} if name == "filter1" && filter.description == "Filter at last level")
        );
        assert!(
            matches!(result_with_ruleset.unwrap(), MatcherConfig::Ruleset {name, ..} if name == "ruleset2")
        );
    }

    #[test]
    fn test_create_node_in_not_valid_path() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "new_filter".to_string(),
                        filter: Default::default(),
                        nodes: vec![],
                    }],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Default::default(),
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![],
                        }],
                    }],
                },
            ],
        };
        let new_filter = MatcherConfig::Filter {
            name: "new_filter".to_string(),
            filter: Default::default(),
            nodes: vec![],
        };

        // Act
        let result_not_existing =
            config.create_node_in_path(&["root", "filter3"], new_filter.clone());
        let result_ruleset = config
            .create_node_in_path(&["root", "filter2", "filter3", "ruleset1"], new_filter.clone());
        let result_already_existing_node =
            config.create_node_in_path(&["root", "filter1"], new_filter);

        // Assert
        assert!(result_not_existing.is_err());
        assert_eq!(
            result_not_existing.err(),
            Some(MatcherError::ConfigurationError {
                message: "Node in this path does not exist: [\"root\", \"filter3\"]".to_string(),
            })
        );
        assert!(result_ruleset.is_err());
        assert_eq!(
            result_ruleset.err(),
            Some(MatcherError::ConfigurationError {
                message: "A ruleset cannot have children nodes".to_string(),
            })
        );
        assert!(result_already_existing_node.is_err());
        assert_eq!(
            result_already_existing_node.err(),
            Some(MatcherError::NotUniqueNameError { name: "new_filter".to_string() })
        );
    }

    #[test]
    fn test_create_node() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Default::default(),
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![],
                        }],
                    }],
                },
            ],
        };
        let expected_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Default::default(),
                        nodes: vec![
                            MatcherConfig::Ruleset { name: "ruleset1".to_string(), rules: vec![] },
                            MatcherConfig::Filter {
                                name: "new_filter".to_string(),
                                filter: Default::default(),
                                nodes: vec![],
                            },
                        ],
                    }],
                },
            ],
        };
        let new_filter = MatcherConfig::Filter {
            name: "new_filter".to_string(),
            filter: Default::default(),
            nodes: vec![],
        };

        // Act
        let result = config.create_node_in_path(&["root", "filter2", "filter3"], new_filter);

        // Assert
        assert!(result.is_ok());
        assert_eq!(config, expected_config);
    }

    #[test]
    fn test_edit_node_in_not_valid_path() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "new_filter".to_string(),
                        filter: Default::default(),
                        nodes: vec![],
                    }],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Default::default(),
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![],
                        }],
                    }],
                },
            ],
        };
        let new_filter = MatcherConfig::Filter {
            name: "edited_filter".to_string(),
            filter: Default::default(),
            nodes: vec![],
        };
        let new_ruleset =
            MatcherConfig::Ruleset { name: "edited_ruleset".to_string(), rules: vec![] };

        // Act
        let result_not_existing =
            config.edit_node_in_path(&["root", "filter3", "new_filter"], new_filter.clone());
        let result_node_different_type =
            config.edit_node_in_path(&["root", "filter2", "filter3"], new_ruleset);

        // Assert
        assert!(result_not_existing.is_err());
        assert_eq!(
            result_not_existing.err(),
            Some(MatcherError::ConfigurationError {
                message: "Node in this path does not exist: [\"root\", \"filter3\"]".to_string(),
            })
        );
        assert!(result_node_different_type.is_err());
        assert_eq!(
            result_node_different_type.err(),
            Some(MatcherError::ConfigurationError {
                message: "Node to edit is not of same type of the new one passed".to_string(),
            })
        );
    }

    #[test]
    fn test_edit_node() {
        // Arrange
        let mut config_ruleset = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Default::default(),
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![Rule { name: "rule1".to_string(), ..Default::default() }],
                        }],
                    }],
                },
            ],
        };
        let mut config_filter = config_ruleset.clone();
        let expected_config_filter = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "edited_filter".to_string(),
                        filter: Default::default(),
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![Rule { name: "rule1".to_string(), ..Default::default() }],
                        }],
                    }],
                },
            ],
        };
        let expected_config_ruleset = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Default::default(),
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "edited_ruleset".to_string(),
                            rules: vec![Rule { name: "rule1".to_string(), ..Default::default() }],
                        }],
                    }],
                },
            ],
        };
        let edited_filter = MatcherConfig::Filter {
            name: "edited_filter".to_string(),
            filter: Default::default(),
            nodes: vec![],
        };
        let edited_ruleset =
            MatcherConfig::Ruleset { name: "edited_ruleset".to_string(), rules: vec![] };

        // Act
        let result_ruleset = config_ruleset
            .edit_node_in_path(&["root", "filter2", "filter3", "ruleset1"], edited_ruleset.clone());
        let result_filter =
            config_filter.edit_node_in_path(&["root", "filter2", "filter3"], edited_filter);

        // Assert
        assert!(result_ruleset.is_ok());
        assert_eq!(config_ruleset, expected_config_ruleset);
        assert!(result_filter.is_ok());
        assert_eq!(config_filter, expected_config_filter);
    }

    #[test]
    fn test_delete_node_not_valid_path() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "new_filter".to_string(),
                        filter: Default::default(),
                        nodes: vec![],
                    }],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Default::default(),
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![],
                        }],
                    }],
                },
            ],
        };

        // Act
        let result_parent_not_existing =
            config.delete_node_in_path(&["root", "filter3", "new_filter"]);
        let result_child_not_existing = config.delete_node_in_path(&["root", "filter2", "filter4"]);

        // Assert
        assert!(result_parent_not_existing.is_err());
        assert_eq!(
            result_parent_not_existing.err(),
            Some(MatcherError::ConfigurationError {
                message: "Node in this path does not exist: [\"root\", \"filter3\"]".to_string(),
            })
        );
        assert!(result_child_not_existing.is_err());
        assert_eq!(
            result_child_not_existing.err(),
            Some(MatcherError::ConfigurationError {
                message: "A node with name \"filter4\" not found in [\"root\", \"filter2\"]"
                    .to_string(),
            })
        );
    }

    #[test]
    fn test_delete_node() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Default::default(),
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![Rule { name: "rule1".to_string(), ..Default::default() }],
                        }],
                    }],
                },
            ],
        };
        let expected_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Filter {
                name: "filter1".to_string(),
                filter: Filter {
                    description: "".to_string(),
                    active: false,
                    filter: Defaultable::Default {},
                },
                nodes: vec![],
            }],
        };

        // Act
        let result = config.delete_node_in_path(&["root", "filter2"]);

        // Assert
        assert!(result.is_ok());
        assert_eq!(config, expected_config);
    }

    #[test]
    fn test_create_rule_in_non_existing_path() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset { name: "ruleset1".to_string(), rules: vec![] }],
        };

        let new_rule = Rule {
            name: "rule-1".to_string(),
            description: "nothing to say here".to_string(),
            do_continue: false,
            active: true,
            constraint: Constraint { where_operator: None, with: Default::default() },
            actions: vec![],
        };

        // Act
        let result = config.create_rule(&["root", "ruleset2"], new_rule);

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.err(),
            Some(MatcherError::ConfigurationError {
                message: "Node in this path does not exist: [\"root\", \"ruleset2\"]".to_string(),
            })
        );
    }

    #[test]
    fn test_create_rule_in_filter() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Filter {
                name: "filter1".to_string(),
                filter: Default::default(),
                nodes: vec![],
            }],
        };

        let new_rule = Rule {
            name: "rule-1".to_string(),
            description: "nothing to say here".to_string(),
            do_continue: false,
            active: true,
            constraint: Constraint { where_operator: None, with: Default::default() },
            actions: vec![],
        };

        // Act
        let result = config.create_rule(&["root", "filter1"], new_rule);

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.err(),
            Some(MatcherError::ConfigurationError {
                message: "Cannot access rules in filter nodes".to_string(),
            })
        );
    }

    #[test]
    fn test_create_rule_already_existing() {
        // Arrange
        let new_rule = Rule {
            name: "rule-1".to_string(),
            description: "nothing to say here".to_string(),
            active: true,
            ..Default::default()
        };
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "ruleset1".to_string(),
                rules: vec![new_rule.clone()],
            }],
        };

        // Act
        let result = config.create_rule(&["root", "ruleset1"], new_rule);

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.err(),
            Some(MatcherError::ConfigurationError {
                message:
                    "A rule with name rule-1 already exists in ruleset [\"root\", \"ruleset1\"]"
                        .to_string(),
            })
        );
    }

    #[test]
    fn test_create_rule() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset { name: "ruleset1".to_string(), rules: vec![] }],
        };

        let new_rule = Rule {
            name: "rule-1".to_string(),
            description: "nothing to say here".to_string(),
            active: true,
            ..Default::default()
        };
        let expected_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "ruleset1".to_string(),
                rules: vec![new_rule.clone()],
            }],
        };

        // Act
        let result = config.create_rule(&["root", "ruleset1"], new_rule);

        // Assert
        assert!(result.is_ok());
        assert_eq!(config, expected_config);
    }

    #[test]
    fn test_edit_rule() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "my-ruleset".to_string(),
                rules: vec![Rule {
                    name: "my-rule".to_string(),
                    description: "My Rule Description".to_string(),
                    do_continue: true,
                    active: true,
                    constraint: Constraint { where_operator: None, with: Default::default() },
                    actions: vec![],
                }],
            }],
        };

        let expected_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "my-ruleset".to_string(),
                rules: vec![Rule {
                    name: "my-rule2".to_string(),
                    description: "My Second Rule Description".to_string(),
                    ..Default::default()
                }],
            }],
        };

        // Act
        let result = config.edit_rule(
            &["root", "my-ruleset"],
            "my-rule",
            Rule {
                name: "my-rule2".to_string(),
                description: "My Second Rule Description".to_string(),
                ..Default::default()
            },
        );

        // Assert
        assert!(result.is_ok());
        assert_eq!(config, expected_config);
    }

    #[test]
    fn test_edit_not_existing_rule() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset { name: "my-ruleset".to_string(), rules: vec![] }],
        };

        let expected_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset { name: "my-ruleset".to_string(), rules: vec![] }],
        };

        // Act
        let result = config.edit_rule(
            &["root", "my-ruleset"],
            "my-rule",
            Rule {
                name: "my-rule2".to_string(),
                description: "My Second Rule Description".to_string(),
                ..Default::default()
            },
        );

        // Assert
        assert!(result.is_err());
        assert_eq!(config, expected_config);
        assert!(matches!(result, Err(MatcherError::ConfigurationError { .. })))
    }

    #[test]
    fn test_edit_rule_in_not_existing_ruleset() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![],
        };

        // Act
        let result = config.edit_rule(
            &["root", "my-ruleset"],
            "my-rule",
            Rule {
                name: "my-rule2".to_string(),
                description: "My Second Rule Description".to_string(),
                ..Default::default()
            },
        );

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(MatcherError::ConfigurationError { .. })))
    }

    #[test]
    fn test_delete_rule() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "my-ruleset".to_string(),
                rules: vec![
                    Rule {
                        name: "my-rule".to_string(),
                        description: "My Rule Description".to_string(),
                        do_continue: true,
                        active: true,
                        constraint: Constraint { where_operator: None, with: Default::default() },
                        actions: vec![],
                    },
                    Rule {
                        name: "my-rule2".to_string(),
                        description: "My Second Rule Description".to_string(),
                        ..Default::default()
                    },
                ],
            }],
        };

        let expected_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "my-ruleset".to_string(),
                rules: vec![Rule {
                    name: "my-rule2".to_string(),
                    description: "My Second Rule Description".to_string(),
                    ..Default::default()
                }],
            }],
        };

        // Act
        let result = config.delete_rule(&["root", "my-ruleset"], "my-rule");

        // Assert
        assert!(result.is_ok());
        assert_eq!(config, expected_config);
    }

    #[test]
    fn test_delete_not_existing_rule() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "my-ruleset".to_string(),
                rules: vec![Rule {
                    name: "my-rule2".to_string(),
                    description: "My Second Rule Description".to_string(),
                    ..Default::default()
                }],
            }],
        };

        let expected_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "my-ruleset".to_string(),
                rules: vec![Rule {
                    name: "my-rule2".to_string(),
                    description: "My Second Rule Description".to_string(),
                    ..Default::default()
                }],
            }],
        };

        // Act
        let result = config.delete_rule(&["root", "my-ruleset"], "my-rule");

        // Assert
        assert!(result.is_err());
        assert_eq!(config, expected_config);
        assert!(matches!(result, Err(MatcherError::ConfigurationError { .. })))
    }

    #[test]
    fn test_delete_rule_in_not_existing_ruleset() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![],
        };

        // Act
        let result = config.delete_rule(&["root", "my-ruleset"], "my-rule");

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(MatcherError::ConfigurationError { .. })))
    }

    fn move_rule_test_ruleset() -> MatcherConfig {
        let default = Rule {
            name: "default-rule".to_string(),
            description: "".to_string(),
            do_continue: true,
            active: true,
            constraint: Constraint { where_operator: None, with: Default::default() },
            actions: vec![],
        };

        MatcherConfig::Ruleset {
            name: "root".to_string(),
            rules: vec![
                Rule { name: "my-rule-001".to_string(), ..default.clone() },
                Rule { name: "my-rule-002".to_string(), ..default.clone() },
                Rule { name: "my-rule-003".to_string(), ..default.clone() },
            ],
        }
    }

    #[test]
    fn test_move_rule_front() {
        // Arrange
        let mut config = move_rule_test_ruleset();

        // Act
        let result = config.move_rule(&["root"], "my-rule-003", 0);

        // Assert
        assert!(result.is_ok());

        let rules = match &config {
            MatcherConfig::Ruleset { rules, .. } => rules,
            config => panic!("{:?}", config),
        };

        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].name, "my-rule-003");
        assert_eq!(rules[1].name, "my-rule-001");
        assert_eq!(rules[2].name, "my-rule-002")
    }

    #[test]
    fn test_move_rule_back() {
        // Arrange
        let mut config = move_rule_test_ruleset();

        // Act
        let result = config.move_rule(&["root"], "my-rule-001", 2);

        // Assert
        assert!(result.is_ok());

        let rules = match &config {
            MatcherConfig::Ruleset { rules, .. } => rules,
            config => panic!("{:?}", config),
        };

        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].name, "my-rule-002");
        assert_eq!(rules[1].name, "my-rule-003");
        assert_eq!(rules[2].name, "my-rule-001")
    }

    #[test]
    fn test_move_rule_repeated() {
        // Arrange
        let mut config = move_rule_test_ruleset();

        // Act
        let result = config.move_rule(&["root"], "my-rule-001", 2);
        let result2 = config.move_rule(&["root"], "my-rule-002", 1);

        // Assert
        assert!(result.is_ok());
        assert!(result2.is_ok());

        let rules = match &config {
            MatcherConfig::Ruleset { rules, .. } => rules,
            config => panic!("{:?}", config),
        };

        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].name, "my-rule-003");
        assert_eq!(rules[1].name, "my-rule-002");
        assert_eq!(rules[2].name, "my-rule-001")
    }

    #[test]
    fn test_move_rule_bounds() {
        let mut config = move_rule_test_ruleset();

        let result = config.move_rule(&["root"], "my-rule-001", 2);
        let result2 = config.move_rule(&["root"], "my-rule-001", 3);

        assert!(result.is_ok());
        assert!(result2.is_err());

        let rules = match &config {
            MatcherConfig::Ruleset { rules, .. } => rules,
            config => panic!("{:?}", config),
        };

        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].name, "my-rule-002");
        assert_eq!(rules[1].name, "my-rule-003");
        assert_eq!(rules[2].name, "my-rule-001")
    }

    #[test]
    fn should_refuse_missformated_names() {
        let filter =
            Filter { description: "".to_string(), active: false, filter: Defaultable::Default {} };

        let old_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: filter.clone(),
            nodes: vec![],
        };
        let mut config = old_config.clone();

        let result = config.create_node_in_path(
            &["root"],
            MatcherConfig::Filter {
                name: "test/name".to_string(),
                filter: filter.clone(),
                nodes: vec![],
            },
        );
        assert!(result.is_err());
        assert_eq!(old_config, config);

        let result = config.create_node_in_path(
            &["root"],
            MatcherConfig::Ruleset { name: "test/name".to_string(), rules: vec![] },
        );
        assert!(result.is_err());
        assert_eq!(old_config, config);
    }

    #[test]
    fn should_refuse_missformated_names_on_edit() {
        let filter =
            Filter { description: "".to_string(), active: false, filter: Defaultable::Default {} };

        let old_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: filter.clone(),
            nodes: vec![],
        };
        let mut config = old_config.clone();

        let result = config.edit_node_in_path(
            &["root"],
            MatcherConfig::Filter {
                name: "test/name".to_string(),
                filter: filter.clone(),
                nodes: vec![],
            },
        );
        assert!(result.is_err());
        assert_eq!(old_config, config);

        let result = config.create_node_in_path(
            &["root"],
            MatcherConfig::Ruleset { name: "test/name".to_string(), rules: vec![] },
        );
        assert!(result.is_err());
        assert_eq!(old_config, config);
    }

    #[test]
    fn should_refuse_missformated_regex_in_rule() {
        let old_config = MatcherConfig::Ruleset { name: "root".to_string(), rules: vec![] };
        let mut config = old_config.clone();

        let result = config.create_rule(
            &["root"],
            Rule {
                name: "test-rule".to_string(),
                constraint: Constraint {
                    where_operator: Some(Operator::Regex {
                        regex: "^(.*$".to_string(),
                        target: "".to_string(),
                    }),
                    with: Default::default(),
                },
                ..Default::default()
            },
        );

        assert!(result.is_err());
        assert_eq!(old_config, config);
    }

    #[test]
    fn should_refuse_missformated_accessor_in_rule() {
        let old_config = MatcherConfig::Ruleset { name: "root".to_string(), rules: vec![] };
        let mut config = old_config.clone();
        let result = config.create_rule(
            &["root"],
            Rule {
                name: "test-rule".to_string(),
                constraint: Constraint {
                    where_operator: Some(Operator::Contains {
                        first: "${pippo}".into(),
                        second: Default::default(),
                    }),
                    with: Default::default(),
                },
                ..Default::default()
            },
        );

        assert!(result.is_err());
        assert_eq!(old_config, config);
    }

    #[test]
    fn should_refuse_missformated_accessor_in_rule_on_edit() {
        let old_config = MatcherConfig::Ruleset {
            name: "root".to_string(),
            rules: vec![Rule { name: "test-rule".to_string(), ..Default::default() }],
        };
        let mut config = old_config.clone();
        let result = config.edit_rule(
            &["root"],
            "test-rule",
            Rule {
                name: "test-rule".to_string(),
                constraint: Constraint {
                    where_operator: Some(Operator::Contains {
                        first: "${pippo}".into(),
                        second: Default::default(),
                    }),
                    with: Default::default(),
                },
                ..Default::default()
            },
        );

        assert!(result.is_err());
        assert_eq!(old_config, config);
    }

    #[test]
    fn should_import_node_in_root() {
        // Arrange
        let mut config = MatcherConfig::Ruleset {
            name: "root".to_string(),
            rules: vec![Rule { name: "test-rule".to_string(), ..Default::default() }],
        };

        let import_config = MatcherConfig::Filter {
            name: "imported_root".to_string(),
            filter: Filter {
                description: "imported root filter".to_string(),
                ..Default::default()
            },
            nodes: vec![],
        };

        // Act
        config.replace_node(&["root"], import_config.clone()).unwrap();

        // Assert
        assert_eq!(config, import_config);
    }

    #[test]
    fn should_import_node_in_path() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter { description: "Root filter".to_string(), ..Default::default() },
            nodes: vec![MatcherConfig::Filter {
                name: "master".to_string(),
                filter: Filter {
                    description: "master filter".to_string(),
                    active: false,
                    filter: Defaultable::Value(Operator::Equals {
                        first: json!("${event.metadata.tenant}"),
                        second: json!("master"),
                    }),
                },
                nodes: vec![
                    MatcherConfig::Filter {
                        name: "filter1".to_string(),
                        filter: Default::default(),
                        nodes: vec![],
                    },
                    MatcherConfig::Filter {
                        name: "filter2".to_string(),
                        filter: Default::default(),
                        nodes: vec![],
                    },
                ],
            }],
        };

        let import_config = MatcherConfig::Filter {
            name: "imported_node".to_string(),
            filter: Filter {
                description: "imported root filter".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![],
        };

        // Act
        config.replace_node(&["root", "master", "filter1"], import_config.clone()).unwrap();
        let new_node = config.get_node_by_path(&["root", "master", "imported_node"]).unwrap();

        // Assert
        assert_eq!(new_node, &import_config);
    }

    #[test]
    fn should_return_error_on_import_to_existing_name() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Filter {
                name: "master".to_string(),
                filter: Default::default(),
                nodes: vec![
                    MatcherConfig::Filter {
                        name: "filter1".to_string(),
                        filter: Default::default(),
                        nodes: vec![],
                    },
                    MatcherConfig::Filter {
                        name: "filter2".to_string(),
                        filter: Default::default(),
                        nodes: vec![],
                    },
                ],
            }],
        };

        // Act
        let result = config.replace_node(
            &["root", "master", "filter2"],
            MatcherConfig::Filter {
                name: "filter1".to_string(),
                filter: Default::default(),
                nodes: vec![],
            },
        );

        // Assert
        match result {
            Err(MatcherError::NotUniqueNameError { name }) if name == "filter1" => {}
            err => unreachable!("{:?}", err),
        }
    }

    #[test]
    fn should_return_error_on_edit_to_existing_name() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Default::default(),
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Default::default(),
                    nodes: vec![],
                },
            ],
        };

        // Act
        let result = config.edit_node_in_path(
            &["root", "filter2"],
            MatcherConfig::Filter {
                name: "filter1".to_string(),
                filter: Default::default(),
                nodes: vec![],
            },
        );

        // Assert
        match result {
            Err(MatcherError::NotUniqueNameError { name }) if name == "filter1" => {}
            err => unreachable!("{:?}", err),
        }
    }

    #[test]
    fn should_find_iterator_ancestor() {
        let config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Iterator {
                name: "iterator".to_string(),
                iterator: Default::default(),
                nodes: vec![MatcherConfig::Ruleset { name: "ruleset".to_string(), rules: vec![] }],
            }],
        };

        let has_iterator_ancestor = config.has_iterator_in_path(&["root", "iterator", "ruleset"]);
        assert!(has_iterator_ancestor);
    }

    #[test]
    fn should_not_find_iterator_ancestor() {
        let config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Default::default(),
            nodes: vec![MatcherConfig::Filter {
                name: "iterator".to_string(),
                filter: Default::default(),
                nodes: vec![MatcherConfig::Ruleset { name: "ruleset".to_string(), rules: vec![] }],
            }],
        };

        let has_iterator_ancestor = config.has_iterator_in_path(&["root", "iterator", "ruleset"]);
        assert!(!has_iterator_ancestor);

        let has_iterator_ancestor = config.has_iterator_in_path(&["root", "pippo", "ruleset"]);
        assert!(!has_iterator_ancestor);
    }

    #[test]
    fn should_delete_child_node_in_iterator() {
        let mut nodes = MatcherConfig::Iterator {
            name: "root".to_string(),
            iterator: Default::default(),
            nodes: vec![MatcherConfig::Ruleset { name: "pippo".to_string(), rules: vec![] }],
        };

        nodes.delete_node_in_path(&["root", "pippo"]).unwrap();

        match nodes {
            MatcherConfig::Iterator { nodes, .. } => {
                assert!(nodes.is_empty())
            }
            result => panic!("{:?}", result),
        }
    }
}
