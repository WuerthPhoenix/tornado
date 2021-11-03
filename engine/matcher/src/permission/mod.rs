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

    fn filter_definition() -> Filter {
        Filter {
            description: "desc".to_owned(),
            active: true,
            filter: Defaultable::Default {},
        }
    }

}