use std::path::Path;
use tornado_engine_matcher::config::v1::fs::FsMatcherConfigManager;
use tornado_engine_matcher::config::v2::{
    gather_dir_entries, get_config_version, FsMatcherConfigManagerV2, Version,
};
use tornado_engine_matcher::config::{MatcherConfigEditor, MatcherConfigReader};

pub async fn upgrade_rules(
    _config_dir: &str,
    rules_dir: &str,
    drafts_dir: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Upgrade Tornado configuration rules");
    upgrade_config(Path::new(rules_dir)).await?;

    let entries = gather_dir_entries(Path::new(drafts_dir)).await?;

    for entry in entries {
        upgrade_config(&entry.path()).await?;
    }

    Ok(())
}

async fn upgrade_config(
    config_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let config_version = get_config_version(Path::new(config_dir)).await?;
    match config_version {
        Version::V1 => upgrade_to_v2(config_dir).await,
        Version::V2 => Ok(()),
    }
}

// ToDo: Improve the error handling in NEPROD-1658
async fn upgrade_to_v2(
    rules_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let config_manager_v1 =
        FsMatcherConfigManager::new(rules_dir.display().to_string().as_str(), "");
    let config_manager_v2 = FsMatcherConfigManagerV2::new(rules_dir, Path::new(""));

    let config = config_manager_v1.get_config().await?;
    config_manager_v2.deploy_config(&config).await?;

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
