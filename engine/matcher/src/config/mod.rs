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
            MatcherConfig::Filter { name, .. } => name,
            MatcherConfig::Ruleset { name, .. } => name,
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

    pub fn get_node_by_path(&self, path: &[&str]) -> Option<&MatcherConfig> {
        // empty path returns None
        if path.is_empty() {
            return None;
        }
        // first element must be current node
        if path[0] != self.get_name() {
            return None;
        }
        let mut root = self;
        // drill down from root
        for &node_name in path[1..].iter() {
            if let Some(new_root) = root.get_child_node_by_name(node_name) {
                root = new_root
            } else {
                return None;
            }
        }
        Some(root)
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
    pub fn create_node_in_path(&self, path: &[&str], node: &MatcherConfig) -> Result<(), MatcherError> {
        // empty path returns None
        if path.is_empty() {
            return Err(MatcherError::ConfigurationError {
                message: format!("Error path empty: [{:?}]", path),
            });
        }
        // first element must be current node
        if path[0] != self.get_name() {
            return Err(MatcherError::ConfigurationError {
                message: format!("First element of path must be the first element in the tree"),
            });
        }
        let mut root = self;
        // drill down from root
        for &node_name in path[1..(path.len() - 1)].iter() {
            if let Some(new_root) = root.get_child_node_by_name(node_name) {
                root = new_root
            } else {
                return Err(MatcherError::ConfigurationError {
                    message: format!("Element not found in tree"),
                });
            }
        }

        match root {
            MatcherConfig::Ruleset { rules: _, .. } => {
                Err(MatcherError::ConfigurationError {
                    message: format!("A ruleset cannot have children nodes"),
                })
            },
            MatcherConfig::Filter { name: _, filter: _, ref mut nodes} => {
                nodes.push(node.clone());
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
}
