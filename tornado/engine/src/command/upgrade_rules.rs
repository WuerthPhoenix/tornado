use crate::config::parse_config_files;
use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigEditor, MatcherConfigReader};

pub async fn upgrade_rules(
    config_dir: &str,
    rules_dir: &str,
    drafts_dir: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Upgrade Tornado configuration rules");
    let configs = parse_config_files(config_dir, rules_dir, drafts_dir)?;
    let mut matcher_config = configs.matcher_config.get_config().await?;
    let matcher_config_clone = matcher_config.clone();
    upgrade(&mut matcher_config)?;
    if matcher_config != matcher_config_clone {
        configs.matcher_config.deploy_config(&matcher_config).await?;
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
        MatcherConfig::Ruleset { .. } => {}
    }
    Ok(())
}

#[cfg(test)]
pub mod test {
    use tempfile::TempDir;

    pub fn prepare_temp_dirs(tempdir: &TempDir) -> (String, String, String) {
        let source_config_dir = "./config/".to_owned();
        let dest_config_dir = tempdir.path().to_str().unwrap().to_owned();

        let mut copy_options = fs_extra::dir::CopyOptions::new();
        copy_options.copy_inside = true;
        fs_extra::dir::copy(source_config_dir, &dest_config_dir, &copy_options).unwrap();

        let draft_dir = "/draft".to_owned();
        let rules_dir = "/rules.d".to_owned();

        (format!("{}/config", dest_config_dir), rules_dir, draft_dir)
    }
}
