use crate::config::filter::Filter;
use crate::config::fs::{FsMatcherConfigManager, ROOT_NODE_NAME};
use crate::config::{
    Defaultable, MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData, MatcherConfigEditor,
    MatcherConfigReader,
};
use crate::error::MatcherError;
use crate::validator::MatcherConfigValidator;
use chrono::Local;
use fs_extra::dir::*;
use log::*;
use std::path::{Path, PathBuf};

const DRAFT_ID: &str = "draft_001";
const DRAFT_CONFIG_DIR: &str = "config";
const DRAFT_DATA_FILE: &str = "data.json";

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

    fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError> {
        debug!("Get draft with id {}", draft_id);

        let config =
            FsMatcherConfigManager::read_from_root_dir(&self.get_draft_config_dir_path(draft_id))?;
        let data = self.read_draft_data(draft_id)?;

        Ok(MatcherConfigDraft { config, data })
    }

    fn create_draft(&self, user: String) -> Result<String, MatcherError> {
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

        let current_ts_ms = current_ts_ms();

        let current_config = self.get_config()?;
        let current_config = match &current_config {
            MatcherConfig::Ruleset { .. } => {
                info!(
                    "A root filter will be automatically added to the created draft with id {}",
                    draft_id
                );
                MatcherConfig::Filter {
                    name: ROOT_NODE_NAME.to_owned(),
                    nodes: vec![current_config],
                    filter: Filter {
                        filter: Defaultable::Default {},
                        description: "".to_owned(),
                        active: true,
                    },
                }
            }
            MatcherConfig::Filter { .. } => current_config,
        };

        FsMatcherConfigManager::matcher_config_to_fs(
            true,
            &self.get_draft_config_dir_path(&draft_id),
            &current_config,
        )?;

        self.write_draft_data(MatcherConfigDraftData {
            user,
            updated_ts_ms: current_ts_ms,
            created_ts_ms: current_ts_ms,
            draft_id: draft_id.clone(),
        })?;

        debug!("Created new draft with id {}", draft_id);
        Ok(draft_id)
    }

    fn update_draft(
        &self,
        draft_id: &str,
        user: String,
        config: &MatcherConfig,
    ) -> Result<(), MatcherError> {
        info!("Update draft with id {}", draft_id);

        MatcherConfigValidator::new().validate(config)?;

        let tempdir = tempfile::tempdir().map_err(|err| MatcherError::InternalSystemError {
            message: format!("Cannot create temporary directory. Err: {}", err),
        })?;
        FsMatcherConfigManager::matcher_config_to_fs(true, tempdir.path(), config)?;

        FsMatcherConfigManager::copy_and_override(
            tempdir.path(),
            &self.get_draft_config_dir_path(&draft_id),
        )?;

        let mut data = self.read_draft_data(draft_id)?;
        data.user = user;
        data.updated_ts_ms = current_ts_ms();

        self.write_draft_data(data)
    }

    fn deploy_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError> {
        info!("Deploy draft with id {}", draft_id);
        let draft_id = DRAFT_ID;
        let draft_config_dir_path = self.get_draft_config_dir_path(draft_id);
        FsMatcherConfigManager::copy_and_override(&draft_config_dir_path, &self.root_path)?;
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

    fn draft_take_over(&self, draft_id: &str, user: String) -> Result<(), MatcherError> {
        info!("User [{}] asks to take over draft with id {}", user, draft_id);
        let mut data = self.read_draft_data(draft_id)?;
        data.user = user;
        self.write_draft_data(data)
    }
}

impl FsMatcherConfigManager {
    fn get_draft_path(&self, draft_id: &str) -> String {
        format!("{}/{}", self.drafts_path, draft_id)
    }

    fn get_draft_config_dir_path(&self, draft_id: &str) -> String {
        format!("{}/{}/{}", self.drafts_path, draft_id, DRAFT_CONFIG_DIR)
    }

    fn get_draft_data_file_path(&self, draft_id: &str) -> String {
        format!("{}/{}/{}", self.drafts_path, draft_id, DRAFT_DATA_FILE)
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
                    let rule_path = current_path.join(&format!("{:09}0_{}.json", index, rule.name));
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

    fn read_draft_data(&self, draft_id: &str) -> Result<MatcherConfigDraftData, MatcherError> {
        let data_json = fs_extra::file::read_to_string(&self.get_draft_data_file_path(draft_id))
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot read data for draft id [{}]. Err: {}", draft_id, err),
            })?;
        serde_json::from_str(&data_json).map_err(|err| MatcherError::ConfigurationError {
            message: format!("Cannot parse JSON data for draft id [{}]. Err: {}", draft_id, err),
        })
    }

    fn write_draft_data(&self, data: MatcherConfigDraftData) -> Result<(), MatcherError> {
        let data_json = serde_json::to_string_pretty(&data).map_err(|err| {
            MatcherError::ConfigurationError {
                message: format!(
                    "Cannot create JSON data for draft id [{}]. Err: {}",
                    data.draft_id, err
                ),
            }
        })?;

        fs_extra::file::write_all(&self.get_draft_data_file_path(&data.draft_id), &data_json)
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot read data for draft id [{}]. Err: {}", data.draft_id, err),
            })
    }
}

fn current_ts_ms() -> i64 {
    Local::now().timestamp_millis()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::MatcherConfigReader;
    use tempfile::TempDir;

    #[test]
    fn should_create_a_new_draft_cloning_from_current_config_with_root_filter(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/config_04");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let current_config = config_manager.get_config().unwrap();

        let user_1 = "user_1".to_owned();

        // Act
        let result = config_manager.create_draft(user_1.clone()).unwrap();
        let draft_config_path = config_manager.get_draft_config_dir_path(&result);

        // Assert
        assert_eq!(DRAFT_ID, &result);
        assert_eq!(
            current_config,
            FsMatcherConfigManager::new(draft_config_path.as_str(), "").get_config()?
        );

        // current_config must be a filter for this test
        match current_config {
            MatcherConfig::Filter { .. } => {}
            _ => assert!(false),
        }

        Ok(())
    }

    #[test]
    fn should_create_a_new_draft_cloning_current_config_with_root_ruleset(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let current_config = config_manager.get_config().unwrap();

        let user_1 = "user_1".to_owned();

        // Act
        let result = config_manager.create_draft(user_1.clone()).unwrap();
        let draft_config_path = config_manager.get_draft_config_dir_path(&result);

        // Assert
        assert_eq!(DRAFT_ID, &result);

        // A default root filter should be automatically added
        match FsMatcherConfigManager::new(draft_config_path.as_str(), "").get_config()? {
            MatcherConfig::Filter { name, nodes, .. } => {
                assert_eq!("root", name);
                assert_eq!(1, nodes.len());
                assert_eq!(current_config, nodes[0]);
            }
            _ => assert!(false),
        }

        // current_config must be a ruleset for this test
        match current_config {
            MatcherConfig::Ruleset { .. } => {}
            _ => assert!(false),
        }

        Ok(())
    }

    #[test]
    fn should_return_a_draft_by_id() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let current_ts_ms = current_ts_ms();
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/config_04");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let current_config = config_manager.get_config().unwrap();

        let user_1 = "user_1".to_owned();

        // Act
        let result = config_manager.create_draft(user_1.clone()).unwrap();
        let draft_content = config_manager.get_draft(&result)?;

        // Assert
        assert_eq!(current_config, draft_content.config);
        assert_eq!(user_1, draft_content.data.user);
        assert!(draft_content.data.created_ts_ms >= current_ts_ms);
        assert_eq!(draft_content.data.updated_ts_ms, draft_content.data.created_ts_ms);

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

        let user_1 = "user_1".to_owned();

        // Act
        let drafts_before_create = config_manager.get_drafts().unwrap();
        let created_draft_id = config_manager.create_draft(user_1).unwrap();
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

        let user_1 = "user_1".to_owned();

        let created_draft_id = config_manager.create_draft(user_1).unwrap();

        // Act
        config_manager.delete_draft(&created_draft_id).unwrap();
        let second_delete_attempt_result = config_manager.delete_draft(&created_draft_id);

        // Assert
        assert!(second_delete_attempt_result.is_err());
        assert!(config_manager.get_drafts().unwrap().is_empty());

        Ok(())
    }

    #[test]
    fn should_save_matcher_config_into_fs() -> Result<(), Box<dyn std::error::Error>> {
        let test_configurations = vec![
            "./test_resources/config_01",
            "./test_resources/config_02",
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

    #[test]
    fn should_update_a_draft_by_id() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);

        let new_config =
            FsMatcherConfigManager::new("./test_resources/config_implicit_filter", drafts_dir)
                .get_config()
                .unwrap();

        let user_1 = "user_1".to_owned();
        let user_2 = "user_2".to_owned();

        // Act
        let draft_id = config_manager.create_draft(user_1.clone()).unwrap();
        let draft_before_update = config_manager.get_draft(&draft_id).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1));

        config_manager.update_draft(&draft_id, user_2.clone(), &new_config).unwrap();
        let draft_after_update = config_manager.get_draft(&draft_id).unwrap();

        // Assert
        assert_eq!(&user_1, &draft_before_update.data.user);
        assert_eq!(&user_2, &draft_after_update.data.user);
        assert_eq!(draft_after_update.data.created_ts_ms, draft_before_update.data.created_ts_ms);
        assert!(draft_after_update.data.updated_ts_ms > draft_before_update.data.updated_ts_ms);
        assert_ne!(draft_after_update.config, draft_before_update.config);
        assert_eq!(new_config, draft_after_update.config);

        Ok(())
    }

    #[test]
    fn should_validate_draft_on_update() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);

        let config_with_invalid_filter_name = MatcherConfig::Filter {
            name: "filter name with space".to_owned(),
            nodes: vec![],
            filter: Filter {
                filter: Defaultable::Default {},
                active: true,
                description: "".to_owned(),
            },
        };

        let config_with_invalid_rule_name = MatcherConfig::Filter {
            name: "filter".to_owned(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "rule name with space".to_owned(),
                rules: vec![],
            }],
            filter: Filter {
                filter: Defaultable::Default {},
                active: true,
                description: "".to_owned(),
            },
        };

        let user_1 = "user_1".to_owned();
        let draft_id = config_manager.create_draft(user_1.clone()).unwrap();

        // Act
        let update_result_1 = config_manager.update_draft(
            &draft_id,
            user_1.clone(),
            &config_with_invalid_filter_name,
        );
        let update_result_2 =
            config_manager.update_draft(&draft_id, user_1.clone(), &config_with_invalid_rule_name);

        // Assert
        assert!(update_result_1.is_err());
        assert!(update_result_2.is_err());
        Ok(())
    }

    #[test]
    fn should_deploy_a_draft_by_id() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let config_before_deploy = config_manager.get_config().unwrap();

        let new_config =
            FsMatcherConfigManager::new("./test_resources/config_implicit_filter", drafts_dir)
                .get_config()
                .unwrap();

        let user_1 = "user_1".to_owned();
        let user_2 = "user_2".to_owned();

        // Act
        let draft_id = config_manager.create_draft(user_2.clone()).unwrap();
        config_manager.update_draft(&draft_id, user_1.clone(), &new_config).unwrap();

        // Act
        let deploy_draft_content = config_manager.deploy_draft(&draft_id).unwrap();
        let config_after_deploy = config_manager.get_config().unwrap();

        // Assert
        assert_ne!(config_before_deploy, config_after_deploy);
        assert_eq!(deploy_draft_content, config_after_deploy);
        assert_eq!(new_config, config_after_deploy);

        Ok(())
    }

    #[test]
    fn should_take_over_a_draft() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules");

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);

        let user_1 = "user_1".to_owned();
        let user_2 = "user_2".to_owned();

        let draft_id = config_manager.create_draft(user_1.clone()).unwrap();

        // Act
        let draft_before_take_over = config_manager.get_draft(&draft_id)?;
        config_manager.draft_take_over(&draft_id, user_2.clone())?;
        let draft_after_take_over = config_manager.get_draft(&draft_id)?;

        // Assert
        assert_eq!(user_1, draft_before_take_over.data.user);
        assert_eq!(user_2, draft_after_take_over.data.user);
        assert_eq!(draft_before_take_over.config, draft_after_take_over.config);

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
