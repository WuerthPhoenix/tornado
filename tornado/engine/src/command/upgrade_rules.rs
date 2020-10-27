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
    use crate::config::MatcherConfigReader;
    use tempfile::TempDir;

    #[test]
    fn should_create_a_new_draft_cloning_from_current_config_with_root_filter() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (config_dir, rules_dir, drafts_dir) = prepare_temp_dirs(&tempdir);

        // Act
        let result = upgrade_rules(&config_dir, &rules_dir, &drafts_dir);

        // Assert
        assert!(result.is_ok());

        Ok(())
    }


    fn prepare_temp_dirs(tempdir: &TempDir) -> (String, String, String) {
        let config_dir = "./config".to_owned();
        let draft_dir = "/draft".to_owned();
        let rules_dir = "/rules".to_owned();

        let source_rules_dir = format!("{}/{}", config_dir, rules_dir);

        let dest_drafts_dir = format!("{}/{}", tempdir.path().to_str().unwrap(), draft_dir);
        let dest_rules_dir = format!("{}/{}", tempdir.path().to_str().unwrap(), rules_dir);

        let mut copy_options = fs_extra::dir::CopyOptions::new();
        copy_options.copy_inside = true;
        fs_extra::dir::copy(&source_rules_dir, &dest_rules_dir, &copy_options).unwrap();
        (config_dir, rules_dir, draft_dir)
    }
}