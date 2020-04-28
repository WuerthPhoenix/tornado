use crate::config::fs::FsMatcherConfigManager;
use crate::config::{MatcherConfig, MatcherConfigEditor, MatcherConfigReader};
use crate::error::MatcherError;
use fs_extra::dir::*;
use log::*;
use std::path::{Path, PathBuf};

const DRAFT_ID: &str = "draft_001";

impl MatcherConfigEditor for FsMatcherConfigManager {
    fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
        let path = Path::new(&self.drafts_path);

        if path.exists() {
            let mut result = vec![];

            for entry in FsMatcherConfigManager::read_dir_entries(path)? {
                let path = entry.path();
                if path.is_dir() {
                    let filename = FsMatcherConfigManager::filename(&path)?;
                    result.push(filename.to_lowercase());
                }
            }

            Ok(result)
        } else {
            Ok(vec![])
        }
    }

    fn get_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError> {
        debug!("Get draft with id {}", draft_id);
        FsMatcherConfigManager::read_from_root_dir(&self.get_draft_path(draft_id))
    }

    fn create_draft(&self) -> Result<String, MatcherError> {
        info!("Create new draft");
        let draft_id = DRAFT_ID.to_owned();
        let draft_path = self.get_draft_path(&draft_id);
        FsMatcherConfigManager::copy_and_override(&self.root_path, &draft_path)?;
        debug!("Created new draft with id {}", draft_id);
        Ok(draft_id)
    }

    fn update_draft(&self, draft_id: &str, config: MatcherConfig) -> Result<(), MatcherError> {
        info!("Update draft with id {}", draft_id);

        let tempdir = tempfile::tempdir().map_err(|err| MatcherError::InternalSystemError {
            message: format!("Cannot create temporary directory. Err: {}", err),
        })?;
        FsMatcherConfigManager::matcher_config_to_fs(true, tempdir.path(), &config)?;

        let draft_path = self.get_draft_path(&draft_id);
        FsMatcherConfigManager::copy_and_override(tempdir.path(), &draft_path)
    }

    fn deploy_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError> {
        info!("Deploy draft with id {}", draft_id);
        let draft_id = DRAFT_ID.to_owned();
        let draft_path = self.get_draft_path(&draft_id);
        FsMatcherConfigManager::copy_and_override(&draft_path, &self.root_path)?;
        self.get_config()
    }

    fn delete_draft(&self, draft_id: &str) -> Result<(), MatcherError> {
        info!("Delete draft with id {}", draft_id);
        let draft_path = self.get_draft_path(&draft_id);

        if Path::new(&draft_path).exists() {
            std::fs::remove_dir_all(&draft_path).map_err(|err| MatcherError::InternalSystemError {
                message: format!("Cannot delete directory [{}]. Err: {}", draft_path, err),
            })
        } else {
            Err(MatcherError::ConfigurationError {
                message: format!(
                    "Cannot delete draft with id [{}] as it does not exists.",
                    draft_id
                ),
            })
        }
    }
}

impl FsMatcherConfigManager {
    fn get_draft_path(&self, draft_id: &str) -> String {
        format!("{}/{}", self.drafts_path, draft_id)
    }

    fn copy_and_override<S: AsRef<Path>, D: AsRef<Path>>(
        source_dir: S,
        dest_dir: D,
    ) -> Result<(), MatcherError> {
        if dest_dir.as_ref().exists() {
            std::fs::remove_dir_all(dest_dir.as_ref()).map_err(|err| {
                MatcherError::InternalSystemError {
                    message: format!(
                        "Cannot delete directory [{}]. Err: {}",
                        dest_dir.as_ref().display(),
                        err
                    ),
                }
            })?;
        }

        let mut copy_options = CopyOptions::new();
        copy_options.copy_inside = true;
        copy(source_dir.as_ref(), dest_dir.as_ref(), &copy_options)
            .map_err(|err| MatcherError::InternalSystemError {
                message: format!(
                    "Cannot copy configuration from [{}] [{}]. Err: {}",
                    source_dir.as_ref().display(),
                    dest_dir.as_ref().display(),
                    err
                ),
            })
            .map(|_| ())
    }

    fn matcher_config_to_fs<P: AsRef<Path>>(
        is_root_node: bool,
        root_path: P,
        config: &MatcherConfig,
    ) -> Result<(), MatcherError> {
        match config {
            MatcherConfig::Ruleset { name, rules } => {
                let current_path =
                    FsMatcherConfigManager::create_node_dir(is_root_node, root_path, name)?;

                for (index, rule) in rules.iter().enumerate() {
                    let rule_path = current_path.join(&format!("{:12}0_{}.json", index, rule.name));
                    let rule_json = serde_json::to_string_pretty(rule).map_err(|err| {
                        MatcherError::InternalSystemError {
                            message: format!("Cannot convert rule body to JSON. Err: {}", err),
                        }
                    })?;
                    fs_extra::file::write_all(&rule_path, &rule_json).map_err(|err| {
                        MatcherError::InternalSystemError {
                            message: format!("Cannot save JSON rule to filesystem. Err: {}", err),
                        }
                    })?
                }
            }
            MatcherConfig::Filter { name, filter, nodes } => {
                let current_path =
                    FsMatcherConfigManager::create_node_dir(is_root_node, root_path, name)?;

                let filter_json = serde_json::to_string_pretty(filter).map_err(|err| {
                    MatcherError::InternalSystemError {
                        message: format!("Cannot convert filter body to JSON. Err: {}", err),
                    }
                })?;
                fs_extra::file::write_all(&current_path.join("filter.json"), &filter_json)
                    .map_err(|err| MatcherError::InternalSystemError {
                        message: format!("Cannot save JSON filter to filesystem. Err: {}", err),
                    })?;

                for node in nodes {
                    FsMatcherConfigManager::matcher_config_to_fs(false, &current_path, node)?
                }
            }
        }
        Ok(())
    }

    fn create_node_dir<P: AsRef<Path>>(
        is_root_node: bool,
        root_path: P,
        node_name: &str,
    ) -> Result<PathBuf, MatcherError> {
        let current_path = if is_root_node {
            root_path.as_ref().to_path_buf()
        } else {
            root_path.as_ref().join(&node_name)
        };

        std::fs::create_dir_all(&current_path).map_err(|err| {
            MatcherError::InternalSystemError {
                message: format!(
                    "Cannot create directory [{}]. Err: {}",
                    current_path.display(),
                    err
                ),
            }
        })?;

        Ok(current_path)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::MatcherConfigReader;
    use tempfile::TempDir;

    #[test]
    fn should_create_a_new_draft_cloning_from_rules_dir() -> Result<(), Box<dyn std::error::Error>>
    {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let current_config = config_manager.get_config().unwrap();

        // Act
        let result = config_manager.create_draft().unwrap();
        let draft_path = config_manager.get_draft_path(&result);

        // Assert
        assert_eq!(DRAFT_ID, &result);
        assert_eq!(format!("{}/{}", drafts_dir, DRAFT_ID), draft_path);
        assert_eq!(
            current_config,
            FsMatcherConfigManager::new(draft_path.as_str(), "").get_config()?
        );

        Ok(())
    }

    #[test]
    fn should_return_a_draft_by_id() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let current_config = config_manager.get_config().unwrap();

        // Act
        let result = config_manager.create_draft().unwrap();
        let draft_content = config_manager.get_draft(&result)?;

        // Assert
        assert_eq!(current_config, draft_content);

        Ok(())
    }

    #[test]
    fn get_draft_should_return_error_if_draft_id_does_not_exists(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);

        // Act
        let result = config_manager.get_draft("Hello, World!");

        // Assert
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn get_drafts_should_return_all_draft_ids() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);

        // Act
        let drafts_before_create = config_manager.get_drafts().unwrap();
        let created_draft_id = config_manager.create_draft().unwrap();
        let drafts_after_create = config_manager.get_drafts().unwrap();
        config_manager.delete_draft(&created_draft_id).unwrap();
        let drafts_after_delete = config_manager.get_drafts().unwrap();

        // Assert
        assert!(drafts_before_create.is_empty());
        assert_eq!(vec![created_draft_id], drafts_after_create);
        assert!(drafts_after_delete.is_empty());

        Ok(())
    }

    #[test]
    fn should_return_delete_a_draft_by_id() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let created_draft_id = config_manager.create_draft().unwrap();

        // Act
        config_manager.delete_draft(&created_draft_id).unwrap();
        let second_delete_attempt_result = config_manager.delete_draft(&created_draft_id);

        // Assert
        assert!(second_delete_attempt_result.is_err());
        assert!(config_manager.get_drafts().unwrap().is_empty());

        Ok(())
    }

    #[test]
    fn should_deploy_a_draft_by_id() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let config_before_deploy = config_manager.get_config().unwrap();

        let new_rules_path = "./test_resources/config_implicit_filter";

        // Copy a different config into the draft
        FsMatcherConfigManager::copy_and_override(
            new_rules_path,
            &config_manager.get_draft_path(DRAFT_ID),
        )
        .unwrap();

        // Act
        let deploy_draft_content = config_manager.deploy_draft(DRAFT_ID).unwrap();
        let config_after_deploy = config_manager.get_config().unwrap();

        // Assert
        assert_ne!(config_before_deploy, config_after_deploy);
        assert_eq!(deploy_draft_content, config_after_deploy);
        assert_eq!(
            FsMatcherConfigManager::new(new_rules_path, drafts_dir).get_config().unwrap(),
            config_after_deploy
        );

        Ok(())
    }

    #[test]
    fn should_save_matcher_config_into_fs() -> Result<(), Box<dyn std::error::Error>> {
        let test_configurations = vec![
            "./test_resources/config_01",
            "./test_resources/config_03",
            "./test_resources/config_04",
            "./test_resources/config_empty",
            "./test_resources/config_implicit_filter",
            "./test_resources/rules",
        ];

        for test_configuration in test_configurations {
            // Arrange
            let tempdir = tempfile::tempdir()?;
            let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, test_configuration);
            let converted_matcher_config_path = tempdir.path().join("matcher_config_to_fs");

            // Act
            let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
            let src_config = config_manager.get_config().unwrap();

            FsMatcherConfigManager::matcher_config_to_fs(
                true,
                &converted_matcher_config_path,
                &src_config,
            )
            .unwrap();

            let config_manager = FsMatcherConfigManager::new(
                converted_matcher_config_path.to_str().unwrap(),
                drafts_dir,
            );
            let converted_config = config_manager.get_config().unwrap();

            // Assert
            assert_eq!(src_config, converted_config);
        }

        Ok(())
    }

    fn prepare_temp_dirs(tempdir: &TempDir, rules_source_dir: &str) -> (String, String) {
        let drafts_dir = format!("{}/drafts", tempdir.path().to_str().unwrap());
        let rules_dir = format!("{}/rules", tempdir.path().to_str().unwrap());

        let mut copy_options = CopyOptions::new();
        copy_options.copy_inside = true;
        copy(rules_source_dir, &rules_dir, &copy_options).unwrap();
        (rules_dir, drafts_dir)
    }
}
