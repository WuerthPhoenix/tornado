use crate::config::MatcherConfig;
use std::collections::HashMap;

pub enum NodeFilter {
    /// All children of the node are accepted
    AllChildren,
    /// No children of the node are accepted
    NoChildren,
    /// Only the selected children of the nodes are accepted
    SelectedChildren(HashMap<String, NodeFilter>),
}

/// Returns a new matcher config that contains only the
/// allowed nodes of the original filter
pub fn matcher_config_filter(matcher_config: &MatcherConfig, filter: &HashMap<String, NodeFilter>) -> Option<MatcherConfig> {
    let node_name = matcher_config.get_name();
    if let Some(node_filter) = filter.get(node_name) {
        match matcher_config {
            MatcherConfig::Ruleset { .. } => Some(matcher_config.clone()),
            MatcherConfig::Filter { name, filter, nodes } => {
                let mut children = vec![];

                match node_filter {
                    NodeFilter::AllChildren => {
                        children = nodes.clone();
                    }
                    NodeFilter::NoChildren => {}
                    NodeFilter::SelectedChildren(selected_children) => {
                        for node in nodes {
                            if let Some(child_node) = matcher_config_filter(node, selected_children) {
                                children.push(child_node)
                            }
                        }
                    }
                }

                Some(MatcherConfig::Filter { name: name.to_owned(), filter: filter.to_owned(), nodes: children })
            },
        }
    } else {
        None
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::config::Defaultable;
    use crate::config::filter::Filter;

    #[test]
    fn filter_should_return_the_none_if_no_matching_name() {
        // Arrange
        let config = MatcherConfig::Filter {
            filter: filter_definition(),
            name: "root".to_owned(),
            nodes: vec![
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1".to_owned(),
                    nodes: vec![
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_1_1".to_owned(),
                            nodes: vec![]
                        }
                    ]
                }
            ]
        };

        let filter = HashMap::from([
            ("other".to_owned(), NodeFilter::AllChildren)
        ]);

        // Act
        let filtered_config = matcher_config_filter(&config, &filter);

        // Assert
        assert!(filtered_config.is_none());
    }

    #[test]
    fn filter_should_return_the_none_if_empty_filter() {
        // Arrange
        let config = MatcherConfig::Filter {
            filter: filter_definition(),
            name: "root".to_owned(),
            nodes: vec![
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1".to_owned(),
                    nodes: vec![
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_1_1".to_owned(),
                            nodes: vec![]
                        }
                    ]
                }
            ]
        };

        let filter = HashMap::new();

        // Act
        let filtered_config = matcher_config_filter(&config, &filter);

        // Assert
        assert!(filtered_config.is_none());
    }

    #[test]
    fn filter_should_return_the_whole_matcher_config() {
        // Arrange
        let config = MatcherConfig::Filter {
            filter: filter_definition(),
            name: "root".to_owned(),
            nodes: vec![
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1".to_owned(),
                    nodes: vec![
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_1_1".to_owned(),
                            nodes: vec![]
                        }
                    ]
                }
            ]
        };

        let filter = HashMap::from([
            ("root".to_owned(), NodeFilter::AllChildren)
        ]);

        // Act
        let filtered_config = matcher_config_filter(&config, &filter);

        // Assert
        assert_eq!(Some(config), filtered_config);
    }

    #[test]
    fn filter_should_return_only_root() {
        // Arrange
        let config = MatcherConfig::Filter {
            filter: filter_definition(),
            name: "root".to_owned(),
            nodes: vec![
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1".to_owned(),
                    nodes: vec![
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_1_1".to_owned(),
                            nodes: vec![]
                        }
                    ]
                }
            ]
        };

        let filter = HashMap::from([
            ("root".to_owned(), NodeFilter::NoChildren)
        ]);

        // Act
        let filtered_config = matcher_config_filter(&config, &filter);

        // Assert
        assert_eq!(Some(MatcherConfig::Filter {
            filter: filter_definition(),
            name: "root".to_owned(),
            nodes: vec![]
        }), filtered_config);
    }

    #[test]
    fn filter_should_return_only_selected_nodes() {
        // Arrange
        let config = MatcherConfig::Filter {
            filter: filter_definition(),
            name: "root".to_owned(),
            nodes: vec![
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1".to_owned(),
                    nodes: vec![
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_1_1".to_owned(),
                            nodes: vec![
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_1_1_1".to_owned(),
                                    nodes: vec![]
                                },
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_1_1_2".to_owned(),
                                    nodes: vec![]
                                }
                            ]
                        },
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_1_2".to_owned(),
                            nodes: vec![
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_1_2_1".to_owned(),
                                    nodes: vec![]
                                },
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_1_2_2".to_owned(),
                                    nodes: vec![]
                                }
                            ]
                        }
                    ]
                },
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_2".to_owned(),
                    nodes: vec![
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_2_1".to_owned(),
                            nodes: vec![
                                MatcherConfig::Ruleset {
                                    name: "child_2_1_1".to_owned(),
                                    rules: vec![]
                                },
                            ]
                        },
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_2_2".to_owned(),
                            nodes: vec![
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_2_2_1".to_owned(),
                                    nodes: vec![]
                                },
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_2_2_2".to_owned(),
                                    nodes: vec![]
                                }
                            ]
                        }
                    ]
                },
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_3".to_owned(),
                    nodes: vec![
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_3_1".to_owned(),
                            nodes: vec![]
                        },
                    ]
                },
                MatcherConfig::Ruleset {
                    name: "child_4".to_owned(),
                    rules: vec![]
                },
                MatcherConfig::Ruleset {
                    name: "child_5".to_owned(),
                    rules: vec![]
                },
            ]
        };

        let filter = HashMap::from([
            ("root".to_owned(), NodeFilter::SelectedChildren(
                HashMap::from([
                    ("child_1".to_owned(), NodeFilter::SelectedChildren(
                        HashMap::from([
                            ("child_1_1".to_owned(), NodeFilter::AllChildren),
                            ("child_1_2".to_owned(), NodeFilter::SelectedChildren(
                                HashMap::from([
                                    ("child_1_2_2".to_owned(), NodeFilter::NoChildren)
                                ])
                            ))
                        ])
                    ))
                    ("child_2".to_owned(), NodeFilter::AllChildren),
                    ("child_3".to_owned(), NodeFilter::NoChildren),
                    ("child_4".to_owned(), NodeFilter::AllChildren),
                ])
            ))
        ]);

        // Act
        let filtered_config = matcher_config_filter(&config, &filter);

        // Assert

        let expected_config = MatcherConfig::Filter {
            filter: filter_definition(),
            name: "root".to_owned(),
            nodes: vec![
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1".to_owned(),
                    nodes: vec![
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_1_1".to_owned(),
                            nodes: vec![
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_1_1_1".to_owned(),
                                    nodes: vec![]
                                },
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_1_1_2".to_owned(),
                                    nodes: vec![]
                                }
                            ]
                        },
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_1_2".to_owned(),
                            nodes: vec![
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_1_2_2".to_owned(),
                                    nodes: vec![]
                                }
                            ]
                        }
                    ]
                },
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_2".to_owned(),
                    nodes: vec![
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_2_1".to_owned(),
                            nodes: vec![
                                MatcherConfig::Ruleset {
                                    name: "child_2_1_1".to_owned(),
                                    rules: vec![]
                                },
                            ]
                        },
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_2_2".to_owned(),
                            nodes: vec![
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_2_2_1".to_owned(),
                                    nodes: vec![]
                                },
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_2_2_2".to_owned(),
                                    nodes: vec![]
                                }
                            ]
                        }
                    ]
                },
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_3".to_owned(),
                    nodes: vec![]
                },
                MatcherConfig::Ruleset {
                    name: "child_4".to_owned(),
                    rules: vec![]
                },

            ]
        };

        assert_eq!(Some(config), filtered_config);
    }


    fn filter_definition() -> Filter {
        Filter {
            description: "desc".to_owned(),
            active: true,
            filter: Defaultable::Default {},
        }
    }

}