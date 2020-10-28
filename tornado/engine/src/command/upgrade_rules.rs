use crate::command::daemon::{ACTION_ID_MONITORING, ACTION_ID_SMART_MONITORING_CHECK_RESULT};
use crate::config::parse_config_files;
use tornado_engine_matcher::config::rule::Action;
use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigEditor, MatcherConfigReader};

pub fn upgrade_rules(
    config_dir: &str,
    rules_dir: &str,
    drafts_dir: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Upgrade Tornado configuration rules");
    let configs = parse_config_files(config_dir, rules_dir, drafts_dir)?;
    let mut matcher_config = configs.matcher_config.get_config()?;
    let matcher_config_clone = matcher_config.clone();
    upgrade(&mut matcher_config)?;
    if matcher_config != matcher_config_clone {
        configs.matcher_config.deploy_config(&matcher_config)?;
        println!("Upgrade Tornado configuration rules completed successfully");
    } else {
        println!("Upgrade Tornado configuration rules completed. Nothing to do.");
    }
    Ok(())
}

fn upgrade(
    matcher_config: &mut MatcherConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    match matcher_config {
        MatcherConfig::Filter { name: _, filter: _, nodes } => {
            for node in nodes {
                upgrade(node)?;
            }
        }
        MatcherConfig::Ruleset { name: _, rules } => {
            for rule in rules {
                for action in &mut rule.actions {
                    if action.id == ACTION_ID_MONITORING {
                        println!(
                            "Migrating {} action to {}",
                            ACTION_ID_MONITORING, ACTION_ID_SMART_MONITORING_CHECK_RESULT
                        );
                        match tornado_executor_smart_monitoring_check_result::migration::migrate_from_monitoring(&action.payload) {
                            Ok(migrated_payload) => {
                                *action = Action {
                                    id: ACTION_ID_SMART_MONITORING_CHECK_RESULT.to_owned(),
                                    payload: migrated_payload
                                }
                            },
                            Err(err) => {
                                println!("Error Migrating {} action to {}. Err: {}", ACTION_ID_MONITORING, ACTION_ID_SMART_MONITORING_CHECK_RESULT, err);
                            }
                        };
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn should_migrate_the_monitoring_rules() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let (config_dir, rules_dir, drafts_dir) = prepare_temp_dirs(&tempdir);
        let configs = parse_config_files(&config_dir, &rules_dir, &drafts_dir).unwrap();

        let matcher_config_before = configs.matcher_config.get_config().unwrap();

        // Act
        upgrade_rules(&config_dir, &rules_dir, &drafts_dir).unwrap();

        // Assert
        let matcher_config_after = configs.matcher_config.get_config().unwrap();
        assert_ne!(matcher_config_before, matcher_config_after);

        check_migrated_rules(&matcher_config_before, &matcher_config_after);
    }

    fn prepare_temp_dirs(tempdir: &TempDir) -> (String, String, String) {
        let source_config_dir = "./config/".to_owned();
        let dest_config_dir = tempdir.path().to_str().unwrap().to_owned();

        let mut copy_options = fs_extra::dir::CopyOptions::new();
        copy_options.copy_inside = true;
        fs_extra::dir::copy(&source_config_dir, &dest_config_dir, &copy_options).unwrap();

        let draft_dir = "/draft".to_owned();
        let rules_dir = "/rules.d".to_owned();

        (format!("{}/config", dest_config_dir), rules_dir, draft_dir)
    }

    fn check_migrated_rules(
        matcher_config_before: &MatcherConfig,
        matcher_config_after: &MatcherConfig,
    ) {
        match matcher_config_before {
            MatcherConfig::Filter {
                name: name_before,
                filter: filer_before,
                nodes: before_nodes,
                ..
            } => match matcher_config_after {
                MatcherConfig::Filter {
                    name: name_after,
                    filter: filer_after,
                    nodes: after_nodes,
                    ..
                } => {
                    assert_eq!(name_before, name_after);
                    assert_eq!(filer_before, filer_after);
                    assert_eq!(before_nodes.len(), after_nodes.len());
                    for i in 0..after_nodes.len() {
                        check_migrated_rules(&before_nodes[i], &after_nodes[i]);
                    }
                }
                _ => assert!(false),
            },
            MatcherConfig::Ruleset { name: name_before, rules: before_rules, .. } => {
                match matcher_config_after {
                    MatcherConfig::Ruleset { name: name_after, rules: after_rules, .. } => {
                        assert_eq!(name_before, name_after);
                        assert_eq!(before_rules.len(), after_rules.len());

                        if !before_rules.is_empty() {
                            for i in 0..before_rules.len() {
                                let before_rule = &before_rules[i];
                                let after_rule = &after_rules[i];
                                assert_eq!(before_rule.name, after_rule.name);
                                assert_eq!(before_rule.description, after_rule.description);
                                assert_eq!(before_rule.active, after_rule.active);
                                assert_eq!(before_rule.constraint, after_rule.constraint);
                                assert_eq!(before_rule.do_continue, after_rule.do_continue);
                                assert_eq!(before_rule.actions.len(), after_rule.actions.len());

                                if !before_rule.actions.is_empty() {
                                    for j in 0..before_rule.actions.len() {
                                        let before_action = &before_rule.actions[j];
                                        let after_action = &after_rule.actions[j];
                                        if before_action.id == ACTION_ID_MONITORING {
                                            assert_eq!(
                                                ACTION_ID_SMART_MONITORING_CHECK_RESULT,
                                                after_action.id
                                            );
                                            assert_ne!(before_action, after_action)
                                        } else {
                                            assert_eq!(before_action, after_action);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => assert!(false),
                }
            }
        }
    }
}
