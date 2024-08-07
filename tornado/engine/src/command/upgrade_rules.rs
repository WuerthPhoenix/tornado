use std::path::{Path, PathBuf};
use tornado_engine_matcher::config::v1::fs::FsMatcherConfigManager;
use tornado_engine_matcher::config::v2::{
    gather_dir_entries, get_config_version, FsMatcherConfigManagerV2, Version,
};
use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigEditor, MatcherConfigReader};

pub async fn upgrade_rules(
    config_dir: &str,
    rules_dir: &str,
    drafts_dir: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Upgrade Tornado configuration rules");
    let rules_dir = {
        let mut config_dir = PathBuf::from(config_dir);
        config_dir.push(rules_dir);
        config_dir
    };

    let drafts_dir = {
        let mut config_dir = PathBuf::from(config_dir);
        config_dir.push(drafts_dir);
        config_dir
    };

    let mut upgraded = upgrade_config(&rules_dir).await?;

    if tokio::fs::try_exists(&drafts_dir).await.unwrap_or(false) {
        let entries = gather_dir_entries(&drafts_dir).await?;
        for entry in entries {
            upgraded |= upgrade_draft(&entry.path()).await?;
        }
    }

    if upgraded {
        println!("Everything upgraded and good to go.")
    } else {
        println!("Nothing to upgrade")
    }

    Ok(())
}

async fn upgrade_draft(
    draft_dir: &Path,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let draft_config_dir = {
        let mut draft_dir = draft_dir.to_path_buf();
        draft_dir.push("config");
        draft_dir
    };

    upgrade_config(&draft_config_dir).await
}

async fn upgrade_config(
    config_dir: &Path,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let config_version = get_config_version(Path::new(config_dir)).await?;
    match config_version {
        Version::V1 => {
            upgrade_to_v2(config_dir).await?;
            Ok(true)
        }
        Version::V2 => Ok(false),
    }
}

// ToDo: Improve the error handling in NEPROD-1658
async fn upgrade_to_v2(
    rules_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Migrating config from {}", rules_dir.display());

    let config_manager_v1 =
        FsMatcherConfigManager::new(rules_dir.display().to_string().as_str(), "");

    let config_manager_v2 = FsMatcherConfigManagerV2::new(rules_dir, Path::new(""));

    let config = config_manager_v1.get_config().await?;
    let config = fixup_empty_config(config);
    config_manager_v2.deploy_config(&config).await?;

    Ok(())
}

fn fixup_empty_config(config: MatcherConfig) -> MatcherConfig {
    match config {
        MatcherConfig::Ruleset { name, rules } if name == "root" && rules.is_empty() => {
            // If the root node is a ruleset with no rules, the dir is empty and should be considered a filter.
            MatcherConfig::Filter { name, filter: Default::default(), nodes: vec![] }
        }
        config => config,
    }
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
