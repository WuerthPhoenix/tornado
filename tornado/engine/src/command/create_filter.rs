use crate::config::{parse_config_files, FilterCreateOpt};
use chrono::Local;
use tornado_common::TornadoError;
use tornado_engine_api::auth::WithOwner;
use tornado_engine_matcher::config::nodes::Filter;
use tornado_engine_matcher::config::MatcherConfig;

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
        let mut config_draft = config_manager.get_draft(&draft_id).await?;
        add_filter(&mut config_draft.config, filter_name, filter.clone())?;
        config_manager
            .update_draft(&draft_id, config_draft.get_owner_id().to_owned(), &config_draft.config)
            .await?;
    }

    Ok(())
}

fn add_filter(
    matcher_config: &mut MatcherConfig,
    filter_to_add_name: &str,
    filter_to_add: Filter,
) -> Result<(), TornadoError> {
    dbg!(&matcher_config);
    let MatcherConfig::Filter { nodes, .. } = matcher_config else {
        return Err(TornadoError::ConfigurationError {
            message: format!(
                "Unexpected Node at root level. Node name: {}",
                matcher_config.get_name()
            ),
        });
    };

    let node_with_same_name = nodes.iter_mut().find(|node| node.get_name() == filter_to_add_name);

    match node_with_same_name {
        None => {
            println!(
                "No node found with name: {}. Filter {} will be created.",
                filter_to_add_name, filter_to_add_name
            );
        }
        Some(MatcherConfig::Filter { filter, .. }) if filter.filter == filter_to_add.filter => {
            println!(
                "Filter with name {} already exists and does not need to be updated. Nothing to do.",
                filter_to_add_name
            );
            return Ok(());
        }
        Some(node) => {
            let name = node.get_name_mut();
            *name = node_backup_name(name);
            println!(
                "Node with name {} already exists and needs to be updated. A backup will be created with the name: {}.",
                filter_to_add_name, name
            );
        }
    }

    nodes.push(MatcherConfig::Filter {
        name: filter_to_add_name.to_string(),
        filter: filter_to_add,
        nodes: vec![],
    });
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
    use tornado_engine_matcher::config::rule::{Constraint, Operator, Rule};
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
                    node => panic!("{:?}", node),
                }
            }
            node => panic!("{:?}", node),
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
                first: Value::String("1".to_owned()),
                second: Value::String("1".to_owned()),
            }),
        };

        // Act
        add_filter(&mut matcher_config, filter_to_add_name, filter_to_add).unwrap();

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
            node => panic!("{:?}", node),
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
                first: Value::String("1".to_owned()),
                second: Value::String("1".to_owned()),
            }),
        };

        // Act
        add_filter(&mut matcher_config, filter_to_add_name, filter_to_add).unwrap();

        // Assert
        assert_ne!(matcher_config_before, matcher_config);
        match matcher_config {
            MatcherConfig::Filter { name, filter: _, nodes } => {
                assert_eq!(name, "root");
                assert_eq!(nodes.len(), 5);
                let backup_node = nodes.iter().find(|node| match node {
                    MatcherConfig::Ruleset { name, rules } => {
                        name.starts_with("ruleset_01_backup_") && rules.len() == 8
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
            node => panic!("{:?}", node),
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
                first: Value::String("${event.metadata.tenant_id}".to_owned()),
                second: Value::String("alpha".to_owned()),
            }),
        };

        // Act
        add_filter(&mut matcher_config, filter_to_add_name, filter_to_add).unwrap();

        // Assert
        assert_eq!(matcher_config_before, matcher_config);
    }

    #[tokio::test]
    async fn add_filter_should_return_err_if_root_node_is_a_non_empty_ruleset() {
        // Arrange
        let mut matcher_config = MatcherConfig::Ruleset {
            name: "root".to_string(),
            rules: vec![Rule {
                name: "my rule".to_string(),
                description: "".to_string(),
                do_continue: false,
                active: false,
                constraint: Constraint { where_operator: None, with: Default::default() },
                actions: vec![],
            }],
        };

        let filter_to_add_name = "tenant_id_alpha";
        let filter_to_add = Filter {
            description: "my new filter".to_string(),
            active: true,
            filter: Defaultable::Default {},
        };

        // Act
        let result = add_filter(&mut matcher_config, filter_to_add_name, filter_to_add);

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_add_filter_if_root_is_an_empty_filter() {
        let mut matcher_config = MatcherConfig::Filter {
            name: "root".to_string(),
            filter: Filter {
                description: "".to_string(),
                active: true,
                filter: Defaultable::Default {},
            },
            nodes: vec![],
        };
        let matcher_config_before = matcher_config.clone();

        let filter_to_add_name = "tenant_id_alpha";
        let filter_to_add = Filter {
            description: "my new filter".to_string(),
            active: true,
            filter: Defaultable::Default {},
        };

        // Act
        add_filter(&mut matcher_config, filter_to_add_name, filter_to_add).unwrap();

        // Assert
        assert_ne!(matcher_config_before, matcher_config);
        match matcher_config {
            MatcherConfig::Filter { name, filter, nodes } => {
                assert_eq!(name, "root");
                assert_eq!(
                    filter,
                    Filter {
                        description: "".to_string(),
                        active: true,
                        filter: Defaultable::Default {}
                    }
                );
                assert_eq!(nodes.len(), 1);

                match nodes.first().unwrap() {
                    MatcherConfig::Filter { name, filter: _, nodes } => {
                        assert_eq!(name, filter_to_add_name);
                        assert_eq!(nodes.len(), 0);
                    }
                    _ => unreachable!(),
                };
            }
            node => panic!("{:?}", node),
        }
    }

    #[tokio::test]
    async fn create_filter_should_update_drafts_without_taking_over() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let (config_dir, rules_dir, drafts_dir) = prepare_temp_dirs(&tempdir);
        let configs = parse_config_files(&config_dir, &rules_dir, &drafts_dir).unwrap();
        let draft_user = "original_user";
        let draft_id = configs.matcher_config.create_draft(draft_user.to_owned()).await.unwrap();

        let draft_before = configs.matcher_config.get_draft(&draft_id).await.unwrap();
        let matcher_config_before = configs.matcher_config.get_config().await.unwrap();

        let filter_to_add_name = "my_new_filter";
        let filter_to_add = Filter {
            description: "my new filter".to_string(),
            active: true,
            filter: Defaultable::Value(Operator::Equals {
                first: Value::String("1".to_owned()),
                second: Value::String("1".to_owned()),
            }),
        };

        // Act
        create_filter(
            &config_dir,
            &rules_dir,
            &drafts_dir,
            &FilterCreateOpt {
                name: filter_to_add_name.to_owned(),
                json_definition: serde_json::to_string(&filter_to_add).unwrap(),
            },
        )
        .await
        .unwrap();

        // Assert
        let configs_after = parse_config_files(&config_dir, &rules_dir, &drafts_dir).unwrap();
        assert_ne!(matcher_config_before, configs_after.matcher_config.get_config().await.unwrap());

        let draft_after = configs_after.matcher_config.get_draft(&draft_id).await.unwrap();

        assert_eq!(draft_before.data.user, draft_after.data.user);
        assert_eq!(draft_before.data.draft_id, draft_after.data.draft_id);
        assert_eq!(draft_before.data.created_ts_ms, draft_after.data.created_ts_ms);

        assert_ne!(draft_before.data.updated_ts_ms, draft_after.data.updated_ts_ms);
        assert_ne!(draft_before.config, draft_after.config);
        match draft_after.config {
            MatcherConfig::Filter { name, filter: _, nodes } => {
                assert_eq!(name, "root");
                assert_eq!(nodes.len(), 5);
                let added_node = nodes.iter().find(|node| match node {
                    MatcherConfig::Filter { name, filter, nodes } => {
                        name == filter_to_add_name && filter == &filter_to_add && nodes.is_empty()
                    }
                    _ => false,
                });
                assert!(added_node.is_some());
            }
            node => panic!("{:?}", node),
        }
    }
}
