use crate::config::{parse_config_files, FilterCreateOpt};
use chrono::Local;
use tornado_common::TornadoError;
use tornado_engine_matcher::config::filter::Filter;
use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigEditor, MatcherConfigReader};

const TORNADO_USER: &str = "tornado";

pub async fn create_filter(
    config_dir: &str,
    rules_dir: &str,
    drafts_dir: &str,
    opts: &FilterCreateOpt,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let filter_name = &opts.name;
    let filter_definition = &opts.json_definition;
    println!("Creating Filter with name {}. Filter definition: {}", filter_name, filter_definition);

    let filter = Filter::from_json(filter_definition)?;

    let configs = parse_config_files(config_dir, rules_dir, drafts_dir)?;
    let config_manager = configs.matcher_config;

    println!("Creating filter {} in current configuration.", filter_name);
    let mut current_config = config_manager.get_config().await?;
    add_filter(&mut current_config, filter_name, filter.clone())?;
    config_manager.deploy_config(&current_config).await?;

    let drafts = config_manager.get_drafts().await?;
    for draft_id in drafts {
        println!("Creating filter {} in draft: {}.", filter_name, &draft_id);
        let mut config_draft = config_manager.get_draft(&draft_id).await?.config;
        add_filter(&mut config_draft, filter_name, filter.clone())?;
        config_manager.update_draft(&draft_id, TORNADO_USER.to_string(), &config_draft).await?;
    }

    Ok(())
}

fn add_filter(
    matcher_config: &mut MatcherConfig,
    filter_to_add_name: &str,
    filter_to_add: Filter,
) -> Result<(), TornadoError> {
    match matcher_config {
        MatcherConfig::Filter { name: _, filter: _, nodes } => {
            let node_with_same_name = nodes.iter_mut().find(|node| match node {
                MatcherConfig::Filter { name, .. } => name == filter_to_add_name,
                MatcherConfig::Ruleset { name, .. } => name == filter_to_add_name,
            });
            if let Some(node_with_same_name) = node_with_same_name {
                match node_with_same_name {
                    MatcherConfig::Filter { name, filter, nodes: _ } => {
                        if filter.filter == filter_to_add.filter {
                            println!(
                                "Filter with name {} already exists and does not need to be updated. Nothing to do.",
                                filter_to_add_name
                            );
                            return Ok(());
                        }
                        *name = node_backup_name(name);
                        println!(
                            "Filter with name {} already exists and needs to be updated. A backup Filter will be created with the name: {}.",
                            filter_to_add_name, name
                        );
                    }
                    MatcherConfig::Ruleset { name, rules: _ } => {
                        *name = node_backup_name(name);
                        println!(
                            "Node with name {} already exists and needs to be updated. A backup Ruleset will be created with the name: {}.",
                            filter_to_add_name, name
                        );
                    }
                }
            } else {
                println!(
                    "No node found with name: {}. Filter {} will be created.",
                    filter_to_add_name, filter_to_add_name
                );
            }
            nodes.push(MatcherConfig::Filter {
                name: filter_to_add_name.to_string(),
                filter: filter_to_add,
                nodes: vec![],
            })
        }
        MatcherConfig::Ruleset { name, rules } => {
            return Err(TornadoError::ConfigurationError {
                message: format!(
                    "Unexpected ruleset at root level. Ruleset name: {}. Rules: {:?}",
                    name, rules
                ),
            })
        }
    }
    Ok(())
}

fn node_backup_name(name: &str) -> String {
    let backup_node_name = format!("{}_backup_{}", name, Local::now().timestamp_millis());
    backup_node_name
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::command::upgrade_rules::test::prepare_temp_dirs;
    use tornado_common_api::Value;
    use tornado_engine_matcher::config::rule::Operator;
    use tornado_engine_matcher::config::Defaultable;

    #[tokio::test]
    async fn should_add_filter_if_not_existing() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let (config_dir, rules_dir, drafts_dir) = prepare_temp_dirs(&tempdir);
        let configs = parse_config_files(&config_dir, &rules_dir, &drafts_dir).unwrap();

        let mut matcher_config = configs.matcher_config.get_config().await.unwrap();
        let matcher_config_before = matcher_config.clone();
        let filter_to_add_name = "new_filter";
        let filter_to_add = Filter {
            description: "my new filter".to_string(),
            active: true,
            filter: Defaultable::Default {},
        };

        // Act
        add_filter(&mut matcher_config, filter_to_add_name, filter_to_add.clone()).unwrap();

        // Assert
        assert_ne!(matcher_config_before, matcher_config);
        match matcher_config {
            MatcherConfig::Filter { name, filter: _, nodes } => {
                assert_eq!(name, "root");
                assert_eq!(nodes.len(), 5);
                match nodes.get(4).unwrap() {
                    MatcherConfig::Filter { name, filter: resulting_filter, nodes } => {
                        assert_eq!(name, filter_to_add_name);
                        assert_eq!(resulting_filter, &filter_to_add);
                        assert_eq!(nodes, &vec![]);
                    }
                    MatcherConfig::Ruleset { .. } => {
                        assert!(false)
                    }
                }
            }
            MatcherConfig::Ruleset { .. } => assert!(false),
        }
    }

    #[tokio::test]
    async fn should_add_filter_and_backup_the_existing_one_if_different() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let (config_dir, rules_dir, drafts_dir) = prepare_temp_dirs(&tempdir);
        let configs = parse_config_files(&config_dir, &rules_dir, &drafts_dir).unwrap();

        let mut matcher_config = configs.matcher_config.get_config().await.unwrap();
        let matcher_config_before = matcher_config.clone();
        let filter_to_add_name = "tenant_id_alpha";
        let filter_to_add = Filter {
            description: "my new filter".to_string(),
            active: true,
            filter: Defaultable::Value(Operator::Equals {
                first: Value::Text("1".to_owned()),
                second: Value::Text("1".to_owned()),
            }),
        };

        // Act
        add_filter(&mut matcher_config, filter_to_add_name, filter_to_add.clone()).unwrap();

        // Assert
        assert_ne!(matcher_config_before, matcher_config);
        match matcher_config {
            MatcherConfig::Filter { name, filter: _, nodes } => {
                assert_eq!(name, "root");
                assert_eq!(nodes.len(), 5);
                let backup_node = nodes.iter().find(|node| match node {
                    MatcherConfig::Filter { name, .. } => {
                        name.starts_with("tenant_id_alpha_backup_")
                    }
                    _ => false,
                });
                assert_eq!(backup_node.unwrap().get_direct_child_nodes_count(), 1);

                let added_node = nodes.iter().find(|node| match node {
                    MatcherConfig::Filter { name, .. } => name == filter_to_add_name,
                    _ => false,
                });
                assert_eq!(added_node.unwrap().get_direct_child_nodes_count(), 0);
            }
            MatcherConfig::Ruleset { .. } => assert!(false),
        }
    }

    #[tokio::test]
    async fn should_add_filter_and_backup_the_existing_ruleset_with_same_name() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let (config_dir, rules_dir, drafts_dir) = prepare_temp_dirs(&tempdir);
        let configs = parse_config_files(&config_dir, &rules_dir, &drafts_dir).unwrap();

        let mut matcher_config = configs.matcher_config.get_config().await.unwrap();
        let matcher_config_before = matcher_config.clone();
        let filter_to_add_name = "ruleset_01";
        let filter_to_add = Filter {
            description: "my new filter".to_string(),
            active: true,
            filter: Defaultable::Value(Operator::Equals {
                first: Value::Text("1".to_owned()),
                second: Value::Text("1".to_owned()),
            }),
        };

        // Act
        add_filter(&mut matcher_config, filter_to_add_name, filter_to_add.clone()).unwrap();

        // Assert
        assert_ne!(matcher_config_before, matcher_config);
        match matcher_config {
            MatcherConfig::Filter { name, filter: _, nodes } => {
                assert_eq!(name, "root");
                assert_eq!(nodes.len(), 5);
                let backup_node = nodes.iter().find(|node| match node {
                    MatcherConfig::Ruleset { name, rules } => {
                        name.starts_with("ruleset_01_backup_") && rules.len() == 10
                    }
                    _ => false,
                });
                assert!(backup_node.is_some());

                let added_node = nodes.iter().find(|node| match node {
                    MatcherConfig::Filter { name, .. } => name == filter_to_add_name,
                    _ => false,
                });
                assert_eq!(added_node.unwrap().get_direct_child_nodes_count(), 0);
            }
            MatcherConfig::Ruleset { .. } => assert!(false),
        }
    }

    #[tokio::test]
    async fn should_not_modify_config_if_filter_exists() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let (config_dir, rules_dir, drafts_dir) = prepare_temp_dirs(&tempdir);
        let configs = parse_config_files(&config_dir, &rules_dir, &drafts_dir).unwrap();

        let mut matcher_config = configs.matcher_config.get_config().await.unwrap();
        let matcher_config_before = matcher_config.clone();
        let filter_to_add_name = "tenant_id_alpha";
        let filter_to_add = Filter {
            description: "my new filter".to_string(),
            active: true,
            filter: Defaultable::Value(Operator::Equals {
                first: Value::Text("${event.metadata.tenant_id}".to_owned()),
                second: Value::Text("alpha".to_owned()),
            }),
        };

        // Act
        add_filter(&mut matcher_config, filter_to_add_name, filter_to_add.clone()).unwrap();

        // Assert
        assert_eq!(matcher_config_before, matcher_config);
    }
}
