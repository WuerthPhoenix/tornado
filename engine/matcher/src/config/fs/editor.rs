use crate::config::filter::Filter;
use crate::config::fs::{FsMatcherConfigManager, ROOT_NODE_NAME};
use crate::config::{
    Defaultable, MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData, MatcherConfigEditor,
    MatcherConfigReader,
};
use crate::error::MatcherError;
use crate::validator::MatcherConfigValidator;
use chrono::Local;
use log::*;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::{
    fs::{create_dir_all, metadata, remove_dir_all, File},
    io::AsyncWriteExt,
};

const DRAFT_ID: &str = "draft_001";
const DRAFT_CONFIG_DIR: &str = "config";
const DRAFT_DATA_FILE: &str = "data.json";

#[async_trait::async_trait(?Send)]
impl MatcherConfigEditor for FsMatcherConfigManager {
    async fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
        let path = Path::new(&self.drafts_path);

        if exists(path).await {
            let mut result = vec![];

            for entry in FsMatcherConfigManager::read_dir_entries(path).await? {
                let path = entry.path();
                if is_dir(&path).await {
                    let filename = FsMatcherConfigManager::filename(&path)?;
                    result.push(filename.to_lowercase());
                }
            }

            Ok(result)
        } else {
            Ok(vec![])
        }
    }

    async fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError> {
        debug!("Get draft with id {}", draft_id);

        let config = FsMatcherConfigManager::read_from_root_dir(
            (&self.get_draft_config_dir_path(draft_id)).into(),
        )
        .await?;
        let data = self.read_draft_data(draft_id).await?;

        Ok(MatcherConfigDraft { config, data })
    }

    async fn create_draft(&self, user: String) -> Result<String, MatcherError> {
        info!("Create new draft");

        let draft_id = DRAFT_ID.to_owned();

        let draft_path = self.get_draft_path(&draft_id);
        if exists(Path::new(&draft_path)).await {
            remove_dir_all(&draft_path).await.map_err(|err| MatcherError::InternalSystemError {
                message: format!("Cannot delete directory [{}]. Err: {:?}", draft_path, err),
            })?;
        }

        let current_ts_ms = current_ts_ms();

        let current_config = self.get_config().await?;
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
            PathBuf::from(&self.get_draft_config_dir_path(&draft_id)),
            current_config,
        )
        .await?;

        self.write_draft_data(MatcherConfigDraftData {
            user,
            updated_ts_ms: current_ts_ms,
            created_ts_ms: current_ts_ms,
            draft_id: draft_id.clone(),
        })
        .await?;

        debug!("Created new draft with id {}", draft_id);
        Ok(draft_id)
    }

    async fn update_draft(
        &self,
        draft_id: &str,
        user: String,
        config: &MatcherConfig,
    ) -> Result<(), MatcherError> {
        info!("Update draft with id {}", draft_id);

        MatcherConfigValidator::new().validate(config)?;

        let tempdir = tempfile::tempdir().map_err(|err| MatcherError::InternalSystemError {
            message: format!("Cannot create temporary directory. Err: {:?}", err),
        })?;
        FsMatcherConfigManager::matcher_config_to_fs(
            true,
            tempdir.path().to_owned(),
            config.clone(),
        )
        .await?;

        FsMatcherConfigManager::copy_and_override(
            tempdir.path(),
            &self.get_draft_config_dir_path(draft_id),
        )
        .await?;

        let mut data = self.read_draft_data(draft_id).await?;
        data.user = user;
        data.updated_ts_ms = current_ts_ms();

        self.write_draft_data(data).await
    }

    async fn deploy_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError> {
        info!("Deploy draft with id {}", draft_id);
        let draft_id = DRAFT_ID;
        let draft = self.get_draft(draft_id).await?;
        self.deploy_config(&draft.config).await
    }

    async fn delete_draft(&self, draft_id: &str) -> Result<(), MatcherError> {
        info!("Delete draft with id {}", draft_id);
        let draft_path = self.get_draft_path(draft_id);

        if exists(Path::new(&draft_path)).await {
            remove_dir_all(&draft_path).await.map_err(|err| MatcherError::InternalSystemError {
                message: format!("Cannot delete directory [{}]. Err: {:?}", draft_path, err),
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

    async fn draft_take_over(&self, draft_id: &str, user: String) -> Result<(), MatcherError> {
        info!("User [{}] asks to take over draft with id {}", user, draft_id);
        let mut data = self.read_draft_data(draft_id).await?;
        data.user = user;
        self.write_draft_data(data).await
    }

    async fn deploy_config(&self, config: &MatcherConfig) -> Result<MatcherConfig, MatcherError> {
        info!("Deploy new configuration");

        MatcherConfigValidator::new().validate(config)?;

        let tempdir = tempfile::tempdir().map_err(|err| MatcherError::InternalSystemError {
            message: format!("Cannot create temporary directory. Err: {:?}", err),
        })?;
        FsMatcherConfigManager::matcher_config_to_fs(
            true,
            tempdir.path().to_owned(),
            config.clone(),
        )
        .await?;

        FsMatcherConfigManager::copy_and_override(tempdir.path(), &self.root_path).await?;
        self.get_config().await
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

    async fn copy_and_override<S: AsRef<Path>, D: AsRef<Path>>(
        source_dir: S,
        dest_dir: D,
    ) -> Result<(), MatcherError> {
        if exists(dest_dir.as_ref()).await {
            remove_dir_all(dest_dir.as_ref()).await.map_err(|err| {
                MatcherError::InternalSystemError {
                    message: format!(
                        "Cannot delete directory [{}]. Err: {:?}",
                        dest_dir.as_ref().display(),
                        err
                    ),
                }
            })?;
        }

        copy_recursive(source_dir.as_ref().into(), dest_dir.as_ref().into()).await
    }

    fn matcher_config_to_fs(
        is_root_node: bool,
        root_path: PathBuf,
        config: MatcherConfig,
    ) -> Pin<Box<dyn Future<Output = Result<(), MatcherError>>>> {
        Box::pin(async move {
            match config {
                MatcherConfig::Ruleset { name, rules } => {
                    let current_path =
                        FsMatcherConfigManager::create_node_dir(is_root_node, root_path, &name)
                            .await?;

                    for (index, rule) in rules.iter().enumerate() {
                        let rule_path =
                            current_path.join(&format!("{:09}0_{}.json", index, rule.name));
                        let rule_json = serde_json::to_string_pretty(rule).map_err(|err| {
                            MatcherError::InternalSystemError {
                                message: format!(
                                    "Cannot convert rule body to JSON. Err: {:?}",
                                    err
                                ),
                            }
                        })?;
                        write_all(&rule_path, &rule_json).await.map_err(|err| {
                            MatcherError::InternalSystemError {
                                message: format!(
                                    "Cannot save JSON rule to filesystem. Err: {:?}",
                                    err
                                ),
                            }
                        })?
                    }
                }
                MatcherConfig::Filter { name, filter, nodes } => {
                    let current_path =
                        FsMatcherConfigManager::create_node_dir(is_root_node, root_path, &name)
                            .await?;

                    let filter_json = serde_json::to_string_pretty(&filter).map_err(|err| {
                        MatcherError::InternalSystemError {
                            message: format!("Cannot convert filter body to JSON. Err: {:?}", err),
                        }
                    })?;
                    write_all(&current_path.join("filter.json"), &filter_json).await.map_err(
                        |err| MatcherError::InternalSystemError {
                            message: format!(
                                "Cannot save JSON filter to filesystem. Err: {:?}",
                                err
                            ),
                        },
                    )?;

                    for node in nodes {
                        FsMatcherConfigManager::matcher_config_to_fs(
                            false,
                            current_path.clone(),
                            node,
                        )
                        .await?
                    }
                }
            }
            Ok(())
        })
    }

    async fn create_node_dir<P: AsRef<Path>>(
        is_root_node: bool,
        root_path: P,
        node_name: &str,
    ) -> Result<PathBuf, MatcherError> {
        let current_path = if is_root_node {
            root_path.as_ref().to_path_buf()
        } else {
            root_path.as_ref().join(node_name)
        };

        create_dir_all(&current_path).await.map_err(|err| MatcherError::InternalSystemError {
            message: format!(
                "Cannot create directory [{}]. Err: {:?}",
                current_path.display(),
                err
            ),
        })?;

        Ok(current_path)
    }

    async fn read_draft_data(
        &self,
        draft_id: &str,
    ) -> Result<MatcherConfigDraftData, MatcherError> {
        let data_json = fs_extra::file::read_to_string(self.get_draft_data_file_path(draft_id))
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot read data for draft id [{}]. Err: {:?}", draft_id, err),
            })?;
        serde_json::from_str(&data_json).map_err(|err| MatcherError::ConfigurationError {
            message: format!("Cannot parse JSON data for draft id [{}]. Err: {:?}", draft_id, err),
        })
    }

    async fn write_draft_data(&self, data: MatcherConfigDraftData) -> Result<(), MatcherError> {
        let data_json = serde_json::to_string_pretty(&data).map_err(|err| {
            MatcherError::ConfigurationError {
                message: format!(
                    "Cannot create JSON data for draft id [{}]. Err: {:?}",
                    data.draft_id, err
                ),
            }
        })?;

        write_all(&self.get_draft_data_file_path(&data.draft_id), &data_json).await.map_err(|err| {
            MatcherError::ConfigurationError {
                message: format!(
                    "Cannot read data for draft id [{}]. Err: {:?}",
                    data.draft_id, err
                ),
            }
        })
    }
}

fn current_ts_ms() -> i64 {
    Local::now().timestamp_millis()
}

async fn write_all<P>(path: P, content: &str) -> Result<(), MatcherError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if exists(path).await && !is_file(path).await {
        return Err(MatcherError::InternalSystemError {
            message: format!("Path \"{:?}\" is not a file!", path),
        });
    }

    let mut f = File::create(path).await.map_err(|err| MatcherError::ConfigurationError {
        message: format!("Cannot write data. Err: {:?}", err),
    })?;

    f.write_all(content.as_bytes()).await.map_err(|err| MatcherError::ConfigurationError {
        message: format!("Cannot write data. Err: {:?}", err),
    })?;

    f.flush().await.map_err(|err| MatcherError::ConfigurationError {
        message: format!("Cannot write data. Err: {:?}", err),
    })
}

async fn copy_recursive(source_dir: PathBuf, dest_dir: PathBuf) -> Result<(), MatcherError> {
    tokio::task::spawn_blocking(move || {

        use fs_extra::dir::*;

        let mut copy_options = CopyOptions::new();
        copy_options.copy_inside = true;
        copy(&source_dir, &dest_dir, &copy_options)
            .map_err(|err| MatcherError::InternalSystemError {
                message: format!(
                    "Cannot copy configuration from [{}] [{}]. Err: {:?}",
                    source_dir.display(),
                    dest_dir.display(),
                    err
                ),
            })
            .map(|_| ())
    }).await.map_err(|err| MatcherError::InternalSystemError {
        message: format!("FsMatcherConfigManager - copy_recursive - Cannot execute Tokio internal task. Err: {:?}", err),
    })?
}

#[inline]
pub async fn exists(path: &Path) -> bool {
    metadata(&path).await.is_ok()
}

#[inline]
pub async fn is_file(path: &Path) -> bool {
    metadata(&path).await.map(|metadata| metadata.is_file()).unwrap_or(false)
}

#[inline]
pub async fn is_dir(path: &Path) -> bool {
    metadata(&path).await.map(|metadata| metadata.is_dir()).unwrap_or(false)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::MatcherConfigReader;
    use tempfile::TempDir;

    #[tokio::test]
    async fn should_create_a_new_draft_cloning_from_current_config_with_root_filter(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) =
            &prepare_temp_dirs(&tempdir, "./test_resources/config_04").await;

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let current_config = config_manager.get_config().await.unwrap();

        let user_1 = "user_1".to_owned();

        // Act
        let result = config_manager.create_draft(user_1.clone()).await.unwrap();
        let draft_config_path = config_manager.get_draft_config_dir_path(&result);

        // Assert
        assert_eq!(DRAFT_ID, &result);
        assert_eq!(
            current_config,
            FsMatcherConfigManager::new(draft_config_path.as_str(), "").get_config().await?
        );

        // current_config must be a filter for this test
        match current_config {
            MatcherConfig::Filter { .. } => {}
            _ => unreachable!(),
        }

        Ok(())
    }

    #[tokio::test]
    async fn should_create_a_new_draft_cloning_current_config_with_root_ruleset(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules").await;

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let current_config = config_manager.get_config().await.unwrap();

        let user_1 = "user_1".to_owned();

        // Act
        let result = config_manager.create_draft(user_1.clone()).await.unwrap();
        let draft_config_path = config_manager.get_draft_config_dir_path(&result);

        // Assert
        assert_eq!(DRAFT_ID, &result);

        // A default root filter should be automatically added
        match FsMatcherConfigManager::new(draft_config_path.as_str(), "").get_config().await? {
            MatcherConfig::Filter { name, nodes, .. } => {
                assert_eq!("root", name);
                assert_eq!(1, nodes.len());
                assert_eq!(current_config, nodes[0]);
            }
            _ => unreachable!(),
        }

        // current_config must be a ruleset for this test
        match current_config {
            MatcherConfig::Ruleset { .. } => {}
            _ => unreachable!(),
        }

        Ok(())
    }

    #[tokio::test]
    async fn should_return_a_draft_by_id() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let current_ts_ms = current_ts_ms();
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) =
            &prepare_temp_dirs(&tempdir, "./test_resources/config_04").await;

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let current_config = config_manager.get_config().await.unwrap();

        let user_1 = "user_1".to_owned();

        // Act
        let result = config_manager.create_draft(user_1.clone()).await.unwrap();
        let draft_content = config_manager.get_draft(&result).await?;

        // Assert
        assert_eq!(current_config, draft_content.config);
        assert_eq!(user_1, draft_content.data.user);
        assert!(draft_content.data.created_ts_ms >= current_ts_ms);
        assert_eq!(draft_content.data.updated_ts_ms, draft_content.data.created_ts_ms);

        Ok(())
    }

    #[tokio::test]
    async fn get_draft_should_return_error_if_draft_id_does_not_exists(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules").await;

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);

        // Act
        let result = config_manager.get_draft("Hello, World!").await;

        // Assert
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn get_drafts_should_return_all_draft_ids() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules").await;

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);

        let user_1 = "user_1".to_owned();

        // Act
        let drafts_before_create = config_manager.get_drafts().await.unwrap();
        let created_draft_id = config_manager.create_draft(user_1).await.unwrap();
        let drafts_after_create = config_manager.get_drafts().await.unwrap();
        config_manager.delete_draft(&created_draft_id).await.unwrap();
        let drafts_after_delete = config_manager.get_drafts().await.unwrap();

        // Assert
        assert!(drafts_before_create.is_empty());
        assert_eq!(vec![created_draft_id], drafts_after_create);
        assert!(drafts_after_delete.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn should_return_delete_a_draft_by_id() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules").await;

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);

        let user_1 = "user_1".to_owned();

        let created_draft_id = config_manager.create_draft(user_1).await.unwrap();

        // Act
        config_manager.delete_draft(&created_draft_id).await.unwrap();
        let second_delete_attempt_result = config_manager.delete_draft(&created_draft_id).await;

        // Assert
        assert!(second_delete_attempt_result.is_err());
        assert!(config_manager.get_drafts().await.unwrap().is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn should_save_matcher_config_into_fs() -> Result<(), Box<dyn std::error::Error>> {
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
            let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, test_configuration).await;
            let converted_matcher_config_path = tempdir.path().join("matcher_config_to_fs");

            // Act
            let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
            let src_config = config_manager.get_config().await.unwrap();

            FsMatcherConfigManager::matcher_config_to_fs(
                true,
                PathBuf::from(&converted_matcher_config_path),
                src_config.clone(),
            )
            .await
            .unwrap();

            let config_manager = FsMatcherConfigManager::new(
                converted_matcher_config_path.to_str().unwrap(),
                drafts_dir,
            );
            let converted_config = config_manager.get_config().await.unwrap();

            // Assert
            assert_eq!(src_config, converted_config);
        }

        Ok(())
    }

    #[tokio::test]
    async fn should_update_a_draft_by_id() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules").await;

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);

        let new_config =
            FsMatcherConfigManager::new("./test_resources/config_implicit_filter", drafts_dir)
                .get_config()
                .await
                .unwrap();

        let user_1 = "user_1".to_owned();
        let user_2 = "user_2".to_owned();

        // Act
        let draft_id = config_manager.create_draft(user_1.clone()).await.unwrap();
        let draft_before_update = config_manager.get_draft(&draft_id).await.unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1));

        config_manager.update_draft(&draft_id, user_2.clone(), &new_config).await.unwrap();
        let draft_after_update = config_manager.get_draft(&draft_id).await.unwrap();

        // Assert
        assert_eq!(&user_1, &draft_before_update.data.user);
        assert_eq!(&user_2, &draft_after_update.data.user);
        assert_eq!(draft_after_update.data.created_ts_ms, draft_before_update.data.created_ts_ms);
        assert!(draft_after_update.data.updated_ts_ms > draft_before_update.data.updated_ts_ms);
        assert_ne!(draft_after_update.config, draft_before_update.config);
        assert_eq!(new_config, draft_after_update.config);

        Ok(())
    }

    #[tokio::test]
    async fn should_validate_draft_on_update() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules").await;

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
        let draft_id = config_manager.create_draft(user_1.clone()).await.unwrap();

        // Act
        let update_result_1 = config_manager
            .update_draft(&draft_id, user_1.clone(), &config_with_invalid_filter_name)
            .await;
        let update_result_2 = config_manager
            .update_draft(&draft_id, user_1.clone(), &config_with_invalid_rule_name)
            .await;

        // Assert
        assert!(update_result_1.is_err());
        assert!(update_result_2.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn should_deploy_a_draft_by_id() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules").await;

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let config_before_deploy = config_manager.get_config().await.unwrap();

        let new_config =
            FsMatcherConfigManager::new("./test_resources/config_implicit_filter", drafts_dir)
                .get_config()
                .await
                .unwrap();

        let user_1 = "user_1".to_owned();
        let user_2 = "user_2".to_owned();

        // Act
        let draft_id = config_manager.create_draft(user_2.clone()).await.unwrap();
        config_manager.update_draft(&draft_id, user_1.clone(), &new_config).await.unwrap();

        // Act
        let deploy_draft_content = config_manager.deploy_draft(&draft_id).await.unwrap();
        let config_after_deploy = config_manager.get_config().await.unwrap();

        // Assert
        assert_ne!(config_before_deploy, config_after_deploy);
        assert_eq!(deploy_draft_content, config_after_deploy);
        assert_eq!(new_config, config_after_deploy);

        Ok(())
    }

    #[tokio::test]
    async fn should_take_over_a_draft() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules").await;

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);

        let user_1 = "user_1".to_owned();
        let user_2 = "user_2".to_owned();

        let draft_id = config_manager.create_draft(user_1.clone()).await.unwrap();

        // Act
        let draft_before_take_over = config_manager.get_draft(&draft_id).await?;
        config_manager.draft_take_over(&draft_id, user_2.clone()).await?;
        let draft_after_take_over = config_manager.get_draft(&draft_id).await?;

        // Assert
        assert_eq!(user_1, draft_before_take_over.data.user);
        assert_eq!(user_2, draft_after_take_over.data.user);
        assert_eq!(draft_before_take_over.config, draft_after_take_over.config);

        Ok(())
    }

    #[tokio::test]
    async fn should_deploy_a_new_config() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let tempdir = tempfile::tempdir()?;
        let (rules_dir, drafts_dir) = &prepare_temp_dirs(&tempdir, "./test_resources/rules").await;

        let config_manager = FsMatcherConfigManager::new(rules_dir, drafts_dir);
        let config_before_deploy = config_manager.get_config().await.unwrap();

        let new_config =
            FsMatcherConfigManager::new("./test_resources/config_implicit_filter", drafts_dir)
                .get_config()
                .await
                .unwrap();

        // Act
        let deployed_config = config_manager.deploy_config(&new_config).await.unwrap();

        // Assert
        let config_after_deploy = config_manager.get_config().await.unwrap();
        assert_ne!(config_before_deploy, config_after_deploy);
        assert_eq!(deployed_config, config_after_deploy);
        assert_eq!(new_config, config_after_deploy);

        Ok(())
    }

    async fn prepare_temp_dirs(tempdir: &TempDir, rules_source_dir: &str) -> (String, String) {
        let drafts_dir = format!("{}/drafts", tempdir.path().to_str().unwrap());
        let rules_dir = format!("{}/rules", tempdir.path().to_str().unwrap());
        copy_recursive(rules_source_dir.into(), (&rules_dir).into()).await.unwrap();
        (rules_dir, drafts_dir)
    }
}
