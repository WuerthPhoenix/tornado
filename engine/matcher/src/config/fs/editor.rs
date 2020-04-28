use crate::config::{MatcherConfigEditor, MatcherConfig};
use crate::config::fs::FsMatcherConfigManager;
use crate::error::MatcherError;
use log::*;
use std::path::Path;
use fs_extra::dir::*;

const DRAFT_ID: &str = "draft_001";

impl MatcherConfigEditor for FsMatcherConfigManager {
    fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
        unimplemented!()
    }

    fn get_draft(&self, draft_id: String) -> Result<MatcherConfig, MatcherError> {
        debug!("Get draft with id {}", draft_id);
        unimplemented!()
    }

    fn create_draft(&self) -> Result<String, MatcherError> {
        info!("Create new draft");
        let draft_id = DRAFT_ID.to_owned();

        let draft_path = self.get_draft_path(&draft_id);

        if Path::new(&draft_path).exists() {
            std::fs::remove_dir_all(&draft_path)
                .map_err(|err| MatcherError::InternalSystemError {
                    message: format!(
                        "Cannot delete directory [{}]. Err: {}",
                        draft_path, err
                    ),
                })?;
        }

        let mut copy_options = CopyOptions::new(); //Initialize default values for CopyOptions
        copy_options.copy_inside = true;
        copy(&self.root_path, &draft_path, &copy_options).map_err(|err| MatcherError::InternalSystemError {
            message: format!(
                "Cannot copy configuration from [{}] [{}]. Err: {}",
                self.root_path, draft_path, err
            ),
        })?;

        debug!("Created new draft with id {}", draft_id);
        Ok(draft_id)
    }

    fn update_draft(&self, draft_id: String, _config: MatcherConfig) -> Result<(), MatcherError> {
        info!("Update draft with id {}", draft_id);
        unimplemented!()
    }

    fn deploy_draft(&self, draft_id: String) -> Result<MatcherConfig, MatcherError> {
        info!("Deploy draft with id {}", draft_id);
        unimplemented!()
    }

    fn delete_draft(&self, draft_id: String) -> Result<(), MatcherError> {
        info!("Delete draft with id {}", draft_id);
        unimplemented!()
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
    use std::fs;
    use crate::config::MatcherConfigReader;

    #[test]
    fn should_create_a_new_draft_cloning_from_rules_dir() -> Result<(), Box<dyn std::error::Error>> {
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
        assert_eq!(current_config, FsMatcherConfigManager::new(draft_path.as_str(), "").get_config()?);

        Ok(())

    }

}