use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::error::MatcherError;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

pub mod filter;
pub mod fs;
pub mod operation;
pub mod rule;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum MatcherConfig {
    Filter { name: String, filter: Filter, nodes: Vec<MatcherConfig> },
    Ruleset { name: String, rules: Vec<Rule> },
}

impl MatcherConfig {
    pub fn get_name(&self) -> &str {
        match self {
            MatcherConfig::Filter { name, .. } | MatcherConfig::Ruleset { name, .. } => name,
        }
    }

    fn get_child_node_by_name(&self, child_name: &str) -> Option<&MatcherConfig> {
        match self {
            MatcherConfig::Filter { nodes, .. } => {
                nodes.iter().find(|child| child.get_name() == child_name)
            }
            MatcherConfig::Ruleset { .. } => None,
        }
    }

    fn get_mut_child_node_by_name(&mut self, child_name: &str) -> Option<&mut MatcherConfig> {
        match self {
            MatcherConfig::Filter { nodes, .. } => {
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

    // Returns child nodes of a node found by a path
    // If the path is empty [], the [self] is returned
    pub fn get_child_nodes_by_path(&self, path: &[&str]) -> Option<Cow<Vec<MatcherConfig>>> {
        if path.is_empty() {
            return Some(Cow::Owned(vec![self.to_owned()]));
        }
        match self.get_node_by_path(path) {
            Some(MatcherConfig::Filter { nodes, .. }) => Some(Cow::Borrowed(nodes)),
            _ => None,
        }
    }

    // Returns the total amount of direct children of a node
    pub fn get_direct_child_nodes_count(&self) -> usize {
        match self {
            MatcherConfig::Filter { nodes, .. } => nodes.len(),
            MatcherConfig::Ruleset { .. } => 0,
        }
    }

    // Returns the total amount of rules of the node and its children
    pub fn get_all_rules_count(&self) -> usize {
        match self {
            MatcherConfig::Filter { nodes, .. } => {
                nodes.iter().map(MatcherConfig::get_all_rules_count).sum()
            }
            MatcherConfig::Ruleset { rules, .. } => rules.len(),
        }
    }

    // Create a node at a specific path
    pub fn create_node_in_path(
        &mut self,
        path: &[&str],
        node: &MatcherConfig,
    ) -> Result<(), MatcherError> {
        if path.len() < 2 {
            return Err(MatcherError::ConfigurationError {
                message: "The node path must specify a parent node".to_string(),
            });
        }
        let path_to_parent = &path[0..path.len() - 1];
        let current_node = self.get_mut_node_by_path_or_err(path_to_parent)?;

        if current_node.get_child_node_by_name(node.get_name()).is_some() {
            return Err(MatcherError::ConfigurationError {
                message: format!(
                    "A node with name {:?} already exists in path {:?}",
                    node.get_name(),
                    path
                ),
            });
        }

        match current_node {
            MatcherConfig::Ruleset { rules: _, .. } => Err(MatcherError::ConfigurationError {
                message: "A ruleset cannot have children nodes".to_string(),
            }),
            MatcherConfig::Filter { name: _, filter: _, ref mut nodes } => {
                nodes.push(node.clone());
                Ok(())
            }
        }
    }

    // Create a node at a specific path
    pub fn edit_node_in_path(
        &mut self,
        path: &[&str],
        node: &MatcherConfig,
    ) -> Result<(), MatcherError> {
        if path.is_empty() {
            return Err(MatcherError::ConfigurationError {
                message: "Empty path is not allowed".to_string(),
            });
        }

        let old_node = self.get_mut_node_by_path_or_err(path)?;
        match (old_node, node) {
            (
                MatcherConfig::Ruleset { name, .. },
                MatcherConfig::Ruleset { name: new_name, .. },
            ) => {
                *name = new_name.clone();
            }
            (
                MatcherConfig::Filter { name, filter, .. },
                MatcherConfig::Filter { name: new_name, filter: new_filter, .. },
            ) => {
                *name = new_name.clone();
                *filter = new_filter.clone();
            }
            _ => {
                return Err(MatcherError::ConfigurationError {
                    message: "Node to edit is not of same type of the new one passed".to_string(),
                });
            }
        }
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

        if parent_node.get_child_node_by_name(node_to_delete).is_none() {
            return Err(MatcherError::ConfigurationError {
                message: format!(
                    "A node with name {:?} not found in {:?}",
                    node_to_delete, path_to_parent,
                ),
            });
        }

        // Parent node is guaranteed to be of type filter because get_child_node_by_name return
        // Option<None> if the parent node is of type ruleset and this match arm is never reached.
        if let MatcherConfig::Filter { nodes, .. } = parent_node {
            nodes.retain(|node| &node.get_name() != node_to_delete);
        }

        Ok(())
    }

    // Create a node at a specific path
    pub fn create_rule(&mut self, ruleset_path: &[&str], rule: Rule) -> Result<(), MatcherError> {
        let node = self.get_mut_node_by_path_or_err(ruleset_path)?;
        let rules = match node {
            MatcherConfig::Filter { .. } => {
                return Err(MatcherError::ConfigurationError {
                    message: "Cannot create rules in filter nodes".to_string(),
                })
            }
            MatcherConfig::Ruleset { rules, .. } => rules,
        };
        if rules.iter().any(|Rule { name, .. }| name == &rule.name) {
            return Err(MatcherError::ConfigurationError {
                message: format!(
                    "A rule with name {} already exists in ruleset {}",
                    rule.name,
                    node.get_name()
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
        let node = self.get_mut_node_by_path_or_err(ruleset_path)?;
        let rules = match node {
            MatcherConfig::Filter { .. } => {
                return Err(MatcherError::ConfigurationError {
                    message: "Cannot edit rules in filter nodes".to_string(),
                })
            }
            MatcherConfig::Ruleset { rules, .. } => rules,
        };

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
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum Defaultable<T: Serialize + Clone> {
    #[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
    Value(T),
    Default {},
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
#[async_trait::async_trait(?Send)]
pub trait MatcherConfigReader: Sync + Send {
    async fn get_config(&self) -> Result<MatcherConfig, MatcherError>;
}

/// A MatcherConfigEditor permits to edit Tornado Configuration drafts
#[async_trait::async_trait(?Send)]
pub trait MatcherConfigEditor: Sync + Send {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rule::Constraint;

    #[test]
    fn test_get_direct_child_nodes_count() {
        // Arrange
        let config_no_children = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![],
        };

        let config_one_children = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![MatcherConfig::Ruleset {
                name: "child_ruleset1".to_string(),
                rules: vec![],
            }],
        };

        let config_more_children = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Ruleset { name: "child_ruleset1".to_string(), rules: vec![] },
                MatcherConfig::Filter {
                    name: "child_filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![],
        };

        let config_no_rules = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![MatcherConfig::Ruleset {
                name: "child_ruleset1".to_string(),
                rules: vec![],
            }],
        };

        let config_one_rules = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Ruleset {
                    name: "child_ruleset1".to_string(),
                    rules: vec![
                        Rule {
                            name: "rule1".to_string(),
                            description: "".to_string(),
                            do_continue: false,
                            active: false,
                            constraint: Constraint {
                                where_operator: None,
                                with: Default::default(),
                            },
                            actions: vec![],
                        },
                        Rule {
                            name: "rule2".to_string(),
                            description: "".to_string(),
                            do_continue: false,
                            active: false,
                            constraint: Constraint {
                                where_operator: None,
                                with: Default::default(),
                            },
                            actions: vec![],
                        },
                    ],
                },
                MatcherConfig::Ruleset {
                    name: "child_ruleset2".to_string(),
                    rules: vec![Rule {
                        name: "rule3".to_string(),
                        description: "".to_string(),
                        do_continue: false,
                        active: false,
                        constraint: Constraint { where_operator: None, with: Default::default() },
                        actions: vec![],
                    }],
                },
                MatcherConfig::Filter {
                    name: "child_filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Ruleset {
                        name: "child_ruleset3".to_string(),
                        rules: vec![
                            Rule {
                                name: "rule4".to_string(),
                                description: "".to_string(),
                                do_continue: false,
                                active: false,
                                constraint: Constraint {
                                    where_operator: None,
                                    with: Default::default(),
                                },
                                actions: vec![],
                            },
                            Rule {
                                name: "rule5".to_string(),
                                description: "".to_string(),
                                do_continue: false,
                                active: false,
                                constraint: Constraint {
                                    where_operator: None,
                                    with: Default::default(),
                                },
                                actions: vec![],
                            },
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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![],
                        }],
                    }],
                },
            ],
        };

        // Act
        let empty_path = config.get_child_nodes_by_path(&vec![]);
        let one_level = config.get_child_nodes_by_path(&vec!["root"]);
        let nested_levels = config.get_child_nodes_by_path(&vec!["root", "filter2"]);
        let nested_levels_path_with_ruleset =
            config.get_child_nodes_by_path(&vec!["root", "filter2", "filter3", "ruleset1"]);
        let not_existing_path = config.get_child_nodes_by_path(&vec!["foo", "bar"]);

        // Assert
        assert_eq!(empty_path.clone().unwrap().len(), 1);
        assert!(
            matches!(empty_path.unwrap().get(0).unwrap(), MatcherConfig::Filter {name, ..} if name == "root")
        );

        assert_eq!(one_level.clone().unwrap().len(), 2);
        assert!(
            matches!(one_level.clone().unwrap().get(0).unwrap(), MatcherConfig::Filter {name, ..} if name == "filter1")
        );
        assert!(
            matches!(one_level.unwrap().get(1).unwrap(), MatcherConfig::Filter {name, ..} if name == "filter2")
        );

        assert_eq!(nested_levels.clone().unwrap().len(), 1);
        assert!(
            matches!(nested_levels.unwrap().get(0).unwrap(), MatcherConfig::Filter {name, ..} if name == "filter3")
        );

        assert_eq!(nested_levels_path_with_ruleset, None);
        assert_eq!(not_existing_path, None);
    }

    #[test]
    fn test_get_node_by_path() {
        // Arrange
        let config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter2".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
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
        let result_with_empty_path = config.get_node_by_path(&vec![]);
        let result_with_wrong_path = config.get_node_by_path(&vec!["root", "foo"]);
        let result_with_first_level_path = config.get_node_by_path(&vec!["root"]);
        let result_with_filter_same_name =
            config.get_node_by_path(&vec!["root", "filter1", "filter2", "filter1"]);
        let result_with_ruleset =
            config.get_node_by_path(&vec!["root", "filter1", "filter2", "ruleset2"]);

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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "new_filter".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
                        nodes: vec![],
                    }],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![],
        };

        // Act
        let result_not_existing =
            config.create_node_in_path(&["root", "filter3", "new_filter"], &new_filter);
        let result_ruleset = config.create_node_in_path(
            &["root", "filter2", "filter3", "ruleset1", "new_filter"],
            &new_filter,
        );
        let result_already_existing_node =
            config.create_node_in_path(&["root", "filter1", "new_filter"], &new_filter);

        // Assert
        assert!(result_not_existing.is_err());
        assert_eq!(
            result_not_existing.err(),
            Some(MatcherError::ConfigurationError {
                message: format!(
                    "Path to parent node does not exist: [\"root\", \"filter3\", \"new_filter\"]"
                ),
            })
        );
        assert!(result_ruleset.is_err());
        assert_eq!(
            result_ruleset.err(),
            Some(MatcherError::ConfigurationError {
                message: format!("A ruleset cannot have children nodes"),
            })
        );
        assert!(result_already_existing_node.is_err());
        assert_eq!(
            result_already_existing_node.err(),
            Some(MatcherError::ConfigurationError {
                message: format!("A node with name \"new_filter\" already exists in path [\"root\", \"filter1\", \"new_filter\"]"),
            })
        );
    }

    #[test]
    fn test_create_node() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
                        nodes: vec![
                            MatcherConfig::Ruleset { name: "ruleset1".to_string(), rules: vec![] },
                            MatcherConfig::Filter {
                                name: "new_filter".to_string(),
                                filter: Filter {
                                    description: "".to_string(),
                                    active: false,
                                    filter: Defaultable::Default {},
                                },
                                nodes: vec![],
                            },
                        ],
                    }],
                },
            ],
        };
        let new_filter = MatcherConfig::Filter {
            name: "new_filter".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![],
        };

        // Act
        let result =
            config.create_node_in_path(&["root", "filter2", "filter3", "new_filter"], &new_filter);

        // Assert
        assert!(result.is_ok());
        assert_eq!(config, expected_config);
    }

    #[test]
    fn test_edit_node_in_not_valid_path() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "new_filter".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
                        nodes: vec![],
                    }],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![],
        };
        let new_ruleset =
            MatcherConfig::Ruleset { name: "edited_ruleset".to_string(), rules: vec![] };

        // Act
        let result_not_existing =
            config.edit_node_in_path(&["root", "filter3", "new_filter"], &new_filter);
        let result_node_different_type =
            config.edit_node_in_path(&["root", "filter2", "filter3"], &new_ruleset);

        // Assert
        assert!(result_not_existing.is_err());
        assert_eq!(
            result_not_existing.err(),
            Some(MatcherError::ConfigurationError {
                message: format!(
                    "Node to edit not found at path [\"root\", \"filter3\", \"new_filter\"]"
                ),
            })
        );
        assert!(result_node_different_type.is_err());
        assert_eq!(
            result_node_different_type.err(),
            Some(MatcherError::ConfigurationError {
                message: format!("Node to edit is not of same type of the new one passed"),
            })
        );
    }

    #[test]
    fn test_edit_node() {
        // Arrange
        let mut config_ruleset = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![Rule {
                                name: "rule1".to_string(),
                                description: "".to_string(),
                                do_continue: false,
                                active: true,
                                constraint: Constraint {
                                    where_operator: None,
                                    with: Default::default(),
                                },
                                actions: vec![],
                            }],
                        }],
                    }],
                },
            ],
        };
        let mut config_filter = config_ruleset.clone();
        let expected_config_filter = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "edited_filter".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![Rule {
                                name: "rule1".to_string(),
                                description: "".to_string(),
                                do_continue: false,
                                active: true,
                                constraint: Constraint {
                                    where_operator: None,
                                    with: Default::default(),
                                },
                                actions: vec![],
                            }],
                        }],
                    }],
                },
            ],
        };
        let expected_config_ruleset = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "edited_ruleset".to_string(),
                            rules: vec![Rule {
                                name: "rule1".to_string(),
                                description: "".to_string(),
                                do_continue: false,
                                active: true,
                                constraint: Constraint {
                                    where_operator: None,
                                    with: Default::default(),
                                },
                                actions: vec![],
                            }],
                        }],
                    }],
                },
            ],
        };
        let edited_filter = MatcherConfig::Filter {
            name: "edited_filter".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![],
        };
        let edited_ruleset =
            MatcherConfig::Ruleset { name: "edited_ruleset".to_string(), rules: vec![] };

        // Act
        let result_ruleset = config_ruleset
            .edit_node_in_path(&["root", "filter2", "filter3", "ruleset1"], &edited_ruleset);
        let result_filter =
            config_filter.edit_node_in_path(&["root", "filter2", "filter3"], &edited_filter);

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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "new_filter".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
                        nodes: vec![],
                    }],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
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
                message:
                    "Path to parent node does not exist: [\"root\", \"filter3\", \"new_filter\"]"
                        .to_string(),
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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![
                MatcherConfig::Filter {
                    name: "filter1".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![],
                },
                MatcherConfig::Filter {
                    name: "filter2".to_string(),
                    filter: Filter {
                        description: "".to_string(),
                        active: false,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Filter {
                        name: "filter3".to_string(),
                        filter: Filter {
                            description: "".to_string(),
                            active: false,
                            filter: Defaultable::Default {},
                        },
                        nodes: vec![MatcherConfig::Ruleset {
                            name: "ruleset1".to_string(),
                            rules: vec![Rule {
                                name: "rule1".to_string(),
                                description: "".to_string(),
                                do_continue: false,
                                active: true,
                                constraint: Constraint {
                                    where_operator: None,
                                    with: Default::default(),
                                },
                                actions: vec![],
                            }],
                        }],
                    }],
                },
            ],
        };
        let expected_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
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
        let result = config.create_rule(&["root", "ruleset2"], new_rule.clone());

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.err(),
            Some(MatcherError::ConfigurationError {
                message: "Path to parent node does not exist: [\"root\", \"ruleset2\"]".to_string(),
            })
        );
    }

    #[test]
    fn test_create_rule_in_filter() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![MatcherConfig::Filter {
                name: "filter1".to_string(),
                filter: Filter {
                    description: "nothing relevant".to_string(),
                    active: true,
                    filter: Defaultable::Default {},
                },
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
        let result = config.create_rule(&["root", "filter1"], new_rule.clone());

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.err(),
            Some(MatcherError::ConfigurationError {
                message: "Cannot create rules in filter nodes".to_string(),
            })
        );
    }

    #[test]
    fn test_create_rule_already_existing() {
        // Arrange
        let new_rule = Rule {
            name: "rule-1".to_string(),
            description: "nothing to say here".to_string(),
            do_continue: false,
            active: true,
            constraint: Constraint { where_operator: None, with: Default::default() },
            actions: vec![],
        };
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![MatcherConfig::Ruleset {
                name: "ruleset1".to_string(),
                rules: vec![new_rule.clone()],
            }],
        };

        // Act
        let result = config.create_rule(&["root", "ruleset1"], new_rule.clone());

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.err(),
            Some(MatcherError::ConfigurationError {
                message: "A rule with name rule-1 already exists in ruleset ruleset1".to_string(),
            })
        );
    }
    #[test]
    fn test_create_rule() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
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
        let expected_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![MatcherConfig::Ruleset {
                name: "ruleset1".to_string(),
                rules: vec![new_rule.clone()],
            }],
        };

        // Act
        let result = config.create_rule(&["root", "ruleset1"], new_rule.clone());

        // Assert
        assert!(result.is_ok());
        assert!(result.is_ok());
        assert_eq!(config, expected_config);
    }

    #[test]
    fn test_edit_rule() {
        // Arrange
        let mut config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![MatcherConfig::Ruleset {
                name: "my-ruleset".to_string(),
                rules: vec![Rule {
                    name: "my-rule2".to_string(),
                    description: "My Second Rule Description".to_string(),
                    do_continue: false,
                    active: false,
                    constraint: Constraint { where_operator: None, with: Default::default() },
                    actions: vec![],
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
                do_continue: false,
                active: false,
                constraint: Constraint { where_operator: None, with: Default::default() },
                actions: vec![],
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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![MatcherConfig::Ruleset { name: "my-ruleset".to_string(), rules: vec![] }],
        };

        let expected_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![MatcherConfig::Ruleset { name: "my-ruleset".to_string(), rules: vec![] }],
        };

        // Act
        let result = config.edit_rule(
            &["root", "my-ruleset"],
            "my-rule",
            Rule {
                name: "my-rule2".to_string(),
                description: "My Second Rule Description".to_string(),
                do_continue: false,
                active: false,
                constraint: Constraint { where_operator: None, with: Default::default() },
                actions: vec![],
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
            filter: Filter {
                description: "".to_string(),
                active: false,
                filter: Defaultable::Default {},
            },
            nodes: vec![],
        };

        // Act
        let result = config.edit_rule(
            &["root", "my-ruleset"],
            "my-rule",
            Rule {
                name: "my-rule2".to_string(),
                description: "My Second Rule Description".to_string(),
                do_continue: false,
                active: false,
                constraint: Constraint { where_operator: None, with: Default::default() },
                actions: vec![],
            },
        );

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(MatcherError::ConfigurationError { .. })))
    }
}
