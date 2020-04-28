use crate::config::fs::FsMatcherConfigManager;
use crate::config::{MatcherConfig, MatcherConfigEditor};
use crate::error::MatcherError;
use fs_extra::dir::*;
use log::*;
use std::path::Path;

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

        if Path::new(&draft_path).exists() {
            std::fs::remove_dir_all(&draft_path).map_err(|err| {
                MatcherError::InternalSystemError {
                    message: format!("Cannot delete directory [{}]. Err: {}", draft_path, err),
                }
            })?;
        }

        let mut copy_options = CopyOptions::new(); //Initialize default values for CopyOptions
        copy_options.copy_inside = true;
        copy(&self.root_path, &draft_path, &copy_options).map_err(|err| {
            MatcherError::InternalSystemError {
                message: format!(
                    "Cannot copy configuration from [{}] [{}]. Err: {}",
                    self.root_path, draft_path, err
                ),
            }
        })?;

        debug!("Created new draft with id {}", draft_id);
        Ok(draft_id)
    }

    fn update_draft(&self, draft_id: &str, _config: MatcherConfig) -> Result<(), MatcherError> {
        info!("Update draft with id {}", draft_id);
        unimplemented!()
    }

    fn deploy_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError> {
        info!("Deploy draft with id {}", draft_id);
        unimplemented!()
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
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::MatcherConfigReader;

    #[test]
    fn should_create_a_new_draft_cloning_from_rules_dir() -> Result<(), Box<dyn std::error::Error>>
    {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let drafts_dir = tempdir.path().to_str().unwrap();
        let rules_dir = "./test_resources/rules";

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
        let drafts_dir = tempdir.path().to_str().unwrap();
        let rules_dir = "./test_resources/rules";

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
        let drafts_dir = tempdir.path().to_str().unwrap();
        let rules_dir = "./test_resources/rules";

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
        let drafts_dir = tempdir.path().to_str().unwrap();
        let rules_dir = "./test_resources/rules";

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
        let drafts_dir = tempdir.path().to_str().unwrap();
        let rules_dir = "./test_resources/rules";

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
}
