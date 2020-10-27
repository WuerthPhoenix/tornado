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
    upgrade(&mut matcher_config)?;
    configs.matcher_config.deploy_config(&matcher_config)?;
    println!("Upgrade Tornado configuration rules completed successfully");
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
                    if &action.id == ACTION_ID_MONITORING {
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

        match matcher_config_before {
            MatcherConfig::Filter {nodes: before_nodes, ..} => {
                match matcher_config_after {
                    MatcherConfig::Filter {nodes: after_nodes, ..} => {
                        assert_eq!(before_nodes.len(), after_nodes.len());

                        let ToDo = 0;
                        // ToDo: check the monitoring rules was migrated
                        assert!(false)
                    },
                    _ => assert!(false)
                }
            },
            _ => assert!(false)
        }
    }

    #[test]
    fn upgrade_fn_should_migrate_the_monitoring_rules() {
        unimplemented!()
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
}