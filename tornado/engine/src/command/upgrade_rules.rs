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
