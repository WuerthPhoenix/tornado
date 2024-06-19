use crate::config::MatcherConfig;
use log::*;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum NodeFilter {
    /// All children of the node are accepted
    AllChildren,
    /// Only the selected children of the nodes are accepted
    SelectedChildren(HashMap<String, NodeFilter>),
}

impl NodeFilter {
    pub fn map_from(paths: &[Vec<String>]) -> HashMap<String, NodeFilter> {
        let mut filters = HashMap::new();
        for path in paths {
            path_to_node_filter(&mut filters, path.as_slice())
        }
        filters
    }
}

fn path_to_node_filter(filters: &mut HashMap<String, NodeFilter>, path: &[String]) {
    if path.is_empty() {
        return;
    }

    let is_last = path.len() == 1;
    let name = path[0].to_owned();

    if is_last {
        filters.insert(name, NodeFilter::AllChildren);
    } else {
        let entries =
            filters.entry(name).or_insert_with(|| NodeFilter::SelectedChildren(HashMap::default()));
        match entries {
            NodeFilter::AllChildren => {}
            NodeFilter::SelectedChildren(children_filters) => {
                path_to_node_filter(children_filters, &path[1..]);
            }
        }
    }
}

/// Returns a new matcher config that contains only the
/// allowed nodes of the original filter
pub fn matcher_config_filter(
    matcher_config: &MatcherConfig,
    filter: &HashMap<String, NodeFilter>,
) -> Option<MatcherConfig> {
    trace!("matcher_config_filter called with matcher_config: {:?}", matcher_config);
    trace!("matcher_config_filter called with filter: {:?}", filter);

    let node_name = matcher_config.get_name();

    let node_filter = filter.get(node_name)?;
    match (node_filter, matcher_config) {
        (_, MatcherConfig::Ruleset { .. }) => Some(matcher_config.clone()),
        (NodeFilter::AllChildren, node) => Some(node.clone()),
        (
            NodeFilter::SelectedChildren(selected_children),
            MatcherConfig::Filter { name, filter, nodes },
        ) => {
            let children: Vec<_> = nodes
                .iter()
                .flat_map(|node| matcher_config_filter(node, selected_children))
                .collect();

            if children.is_empty() {
                return None;
            }
            Some(MatcherConfig::Filter {
                name: name.to_owned(),
                filter: filter.to_owned(),
                nodes: children,
            })
        }
        (
            NodeFilter::SelectedChildren(selected_children),
            MatcherConfig::Iterator { name, target, nodes },
        ) => {
            let children: Vec<_> = nodes
                .iter()
                .flat_map(|node| matcher_config_filter(node, selected_children))
                .collect();

            if children.is_empty() {
                return None;
            }
            Some(MatcherConfig::Iterator {
                name: name.to_owned(),
                target: target.to_owned(),
                nodes: children,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::filter::Filter;
    use crate::config::Defaultable;
    use maplit::hashmap;

    #[test]
    fn filter_should_return_the_none_if_no_matching_name() {
        // Arrange
        let config = MatcherConfig::Filter {
            filter: filter_definition(),
            name: "root".to_owned(),
            nodes: vec![MatcherConfig::Filter {
                filter: filter_definition(),
                name: "child_1".to_owned(),
                nodes: vec![MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1_1".to_owned(),
                    nodes: vec![],
                }],
            }],
        };

        let mut filter = HashMap::new();
        filter.insert("other".to_owned(), NodeFilter::AllChildren);

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
            nodes: vec![MatcherConfig::Filter {
                filter: filter_definition(),
                name: "child_1".to_owned(),
                nodes: vec![MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1_1".to_owned(),
                    nodes: vec![],
                }],
            }],
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
            nodes: vec![MatcherConfig::Filter {
                filter: filter_definition(),
                name: "child_1".to_owned(),
                nodes: vec![MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1_1".to_owned(),
                    nodes: vec![],
                }],
            }],
        };

        let mut filter = HashMap::new();
        filter.insert("root".to_owned(), NodeFilter::AllChildren);

        // Act
        let filtered_config = matcher_config_filter(&config, &filter);

        // Assert
        assert_eq!(Some(config), filtered_config);
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
                                    nodes: vec![],
                                },
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_1_1_2".to_owned(),
                                    nodes: vec![],
                                },
                            ],
                        },
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_1_2".to_owned(),
                            nodes: vec![
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_1_2_1".to_owned(),
                                    nodes: vec![],
                                },
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_1_2_2".to_owned(),
                                    nodes: vec![],
                                },
                            ],
                        },
                    ],
                },
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_2".to_owned(),
                    nodes: vec![
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_2_1".to_owned(),
                            nodes: vec![MatcherConfig::Ruleset {
                                name: "child_2_1_1".to_owned(),
                                rules: vec![],
                            }],
                        },
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_2_2".to_owned(),
                            nodes: vec![
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_2_2_1".to_owned(),
                                    nodes: vec![],
                                },
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_2_2_2".to_owned(),
                                    nodes: vec![],
                                },
                            ],
                        },
                    ],
                },
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_3".to_owned(),
                    nodes: vec![MatcherConfig::Filter {
                        filter: filter_definition(),
                        name: "child_3_1".to_owned(),
                        nodes: vec![],
                    }],
                },
                MatcherConfig::Ruleset { name: "child_4".to_owned(), rules: vec![] },
                MatcherConfig::Ruleset { name: "child_5".to_owned(), rules: vec![] },
            ],
        };

        let filter = hashmap![
            "root".to_owned() => NodeFilter::SelectedChildren(hashmap![
                "child_1".to_owned() => NodeFilter::SelectedChildren(hashmap![
                        "child_1_1".to_owned() => NodeFilter::AllChildren,
                        "child_1_2".to_owned() => NodeFilter::SelectedChildren(hashmap!["child_1_2_2".to_owned() => NodeFilter::AllChildren])
                    ]),
                "child_2".to_owned() => NodeFilter::AllChildren,
                "child_4".to_owned() => NodeFilter::AllChildren,
            ],
        )];

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
                                    nodes: vec![],
                                },
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_1_1_2".to_owned(),
                                    nodes: vec![],
                                },
                            ],
                        },
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_1_2".to_owned(),
                            nodes: vec![MatcherConfig::Filter {
                                filter: filter_definition(),
                                name: "child_1_2_2".to_owned(),
                                nodes: vec![],
                            }],
                        },
                    ],
                },
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_2".to_owned(),
                    nodes: vec![
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_2_1".to_owned(),
                            nodes: vec![MatcherConfig::Ruleset {
                                name: "child_2_1_1".to_owned(),
                                rules: vec![],
                            }],
                        },
                        MatcherConfig::Filter {
                            filter: filter_definition(),
                            name: "child_2_2".to_owned(),
                            nodes: vec![
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_2_2_1".to_owned(),
                                    nodes: vec![],
                                },
                                MatcherConfig::Filter {
                                    filter: filter_definition(),
                                    name: "child_2_2_2".to_owned(),
                                    nodes: vec![],
                                },
                            ],
                        },
                    ],
                },
                MatcherConfig::Ruleset { name: "child_4".to_owned(), rules: vec![] },
            ],
        };

        assert_eq!(Some(expected_config), filtered_config);
    }

    #[test]
    fn node_filter_map_from_should_return_empty_map() {
        // Arrange
        let paths = [];

        // Act
        let filter_map = NodeFilter::map_from(&paths);

        // Assert
        assert!(filter_map.is_empty());
    }

    #[test]
    fn node_filter_map_from_should_return_empty_map_for_empty_path() {
        // Arrange
        let paths = vec![vec![]];

        // Act
        let filter_map = NodeFilter::map_from(&paths);

        // Assert
        assert!(filter_map.is_empty());
    }

    #[test]
    fn node_filter_map_from_should_return_root_all_children() {
        // Arrange
        let paths = vec![vec!["root".to_owned()]];

        // Act
        let filter_map = NodeFilter::map_from(&paths);

        // Assert
        let mut expected_filter_map = HashMap::new();
        expected_filter_map.insert("root".to_owned(), NodeFilter::AllChildren);

        assert_eq!(expected_filter_map, filter_map);
    }

    #[test]
    fn node_filter_map_from_should_return_path_from_root() {
        // Arrange
        let paths = vec![vec!["root".to_owned(), "child_1".to_owned(), "child_1_2".to_owned()]];

        // Act
        let filter_map = NodeFilter::map_from(&paths);

        // Assert
        let expected_filter_map = hashmap![
            "root".to_owned() => NodeFilter::SelectedChildren( hashmap![
                "child_1".to_owned() => NodeFilter::SelectedChildren(hashmap![
                    "child_1_2".to_owned() => NodeFilter::AllChildren
                ]),
            ]),
        ];

        assert_eq!(expected_filter_map, filter_map);
    }

    #[test]
    fn node_filter_map_from_should_merge_multiple_paths() {
        // Arrange
        let paths = vec![
            vec!["root".to_owned(), "child_1".to_owned(), "child_1_2".to_owned()],
            vec!["root".to_owned(), "child_1".to_owned()],
            vec![
                "root".to_owned(),
                "child_1".to_owned(),
                "child_1_2".to_owned(),
                "child_1_3".to_owned(),
            ],
            vec!["root".to_owned(), "child_2".to_owned(), "child_2_1".to_owned()],
            vec!["root".to_owned(), "child_3".to_owned()],
            vec!["another_root".to_owned(), "child_1".to_owned()],
        ];

        // Act
        let filter_map = NodeFilter::map_from(&paths);

        // Assert
        let expected_filter_map = hashmap![
            "root".to_owned() => NodeFilter::SelectedChildren(hashmap![
                "child_1".to_owned() => NodeFilter::AllChildren,
                "child_2".to_owned() => NodeFilter::SelectedChildren(hashmap![
                    "child_2_1".to_owned() => NodeFilter::AllChildren,
                ]),
                "child_3".to_owned() => NodeFilter::AllChildren,
            ]),
            "another_root".to_owned() => NodeFilter::SelectedChildren(hashmap![
                "child_1".to_owned() => NodeFilter::AllChildren,
            ]),
        ];

        assert_eq!(expected_filter_map, filter_map);
    }

    #[test]
    fn node_filter_map_from_should_return_root_all_children_from_merged_paths() {
        // Arrange
        let paths = vec![
            vec!["root".to_owned(), "child_1".to_owned()],
            vec!["root".to_owned()],
            vec!["root".to_owned(), "child_2".to_owned()],
        ];

        // Act
        let filter_map = NodeFilter::map_from(&paths);

        // Assert
        let mut expected_filter_map = HashMap::new();
        expected_filter_map.insert("root".to_owned(), NodeFilter::AllChildren);

        assert_eq!(expected_filter_map, filter_map);
    }

    #[test]
    fn should_filter_a_config_starting_from_paths() {
        // Arrange
        let config = MatcherConfig::Filter {
            filter: filter_definition(),
            name: "root".to_owned(),
            nodes: vec![
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1".to_owned(),
                    nodes: vec![MatcherConfig::Filter {
                        filter: filter_definition(),
                        name: "child_1_1".to_owned(),
                        nodes: vec![],
                    }],
                },
                MatcherConfig::Ruleset { name: "hi".to_owned(), rules: vec![] },
            ],
        };

        let paths = vec![
            vec!["root".to_owned(), "child_1".to_owned()],
            vec!["root".to_owned(), "child_2".to_owned()],
        ];

        // Act
        let filter = NodeFilter::map_from(&paths);
        let filtered_config = matcher_config_filter(&config, &filter);

        // Assert
        let expected_config = MatcherConfig::Filter {
            filter: filter_definition(),
            name: "root".to_owned(),
            nodes: vec![MatcherConfig::Filter {
                filter: filter_definition(),
                name: "child_1".to_owned(),
                nodes: vec![MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1_1".to_owned(),
                    nodes: vec![],
                }],
            }],
        };

        assert_eq!(Some(expected_config), filtered_config);
    }

    #[test]
    fn node_filter_map_from_should_return_none_if_not_a_full_path_match() {
        // Arrange
        let config = MatcherConfig::Filter {
            filter: filter_definition(),
            name: "root".to_owned(),
            nodes: vec![
                MatcherConfig::Filter {
                    filter: filter_definition(),
                    name: "child_1".to_owned(),
                    nodes: vec![MatcherConfig::Filter {
                        filter: filter_definition(),
                        name: "child_1_1".to_owned(),
                        nodes: vec![],
                    }],
                },
                MatcherConfig::Ruleset { name: "hi".to_owned(), rules: vec![] },
            ],
        };

        let paths = vec![vec!["root".to_owned(), "child_1".to_owned(), "child_1_2".to_owned()]];

        // Act
        let filter = NodeFilter::map_from(&paths);
        let filtered_config = matcher_config_filter(&config, &filter);

        // Assert
        assert!(filtered_config.is_none());
    }

    fn filter_definition() -> Filter {
        Filter { description: "desc".to_owned(), active: true, filter: Defaultable::Default {} }
    }
}
