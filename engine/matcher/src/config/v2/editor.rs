use crate::config::nodes::{Filter, MatcherIterator};
use crate::config::rule::Rule;
use crate::config::v2::error::DeploymentError;
use crate::config::v2::{
    gather_dir_entries, parse_node_config_from_file, read_config_from_root_dir, ConfigNodeDir,
    FsMatcherConfigManagerV2, MatcherConfigError, MatcherConfigFilter, MatcherConfigIterator,
    MatcherConfigRuleset, Version,
};
use crate::config::{
    v1, MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData, MatcherConfigEditor,
};
use crate::error::MatcherError;
use crate::matcher::Matcher;
use chrono::Local;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{debug, error, info, warn};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

const DRAFT_ID: &str = "draft_001";

#[async_trait::async_trait(?Send)]
impl MatcherConfigEditor for FsMatcherConfigManagerV2 {
    async fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
        Ok(get_drafts(&self.drafts_path).await?)
    }

    async fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError> {
        // ToDo: Do proper sanitation of the draft_id when multitenancy is added to avoid path-traversal vulnerabilities.
        if draft_id != DRAFT_ID {
            return Err(MatcherError::DraftNotFoundError { draft_id: draft_id.to_string() });
        }

        let draft_dir = {
            let mut path = self.drafts_path.to_path_buf();
            path.push(draft_id);
            path
        };

        Ok(get_draft_from_dir(&draft_dir).await?)
    }

    async fn create_draft(&self, user: String) -> Result<String, MatcherError> {
        let draft_path = {
            let mut path = self.drafts_path.to_path_buf();
            path.push(DRAFT_ID);
            path
        };

        create_draft(&self.root_path, &draft_path, &user, DRAFT_ID).await?;
        Ok(DRAFT_ID.to_string())
    }

    async fn update_draft(
        &self,
        draft_id: &str,
        user: String,
        config: &MatcherConfig,
    ) -> Result<(), MatcherError> {
        // ToDo: Do proper sanitation of the draft_id when multitenancy is added to avoid path-traversal vulnerabilities.
        if draft_id != DRAFT_ID {
            return Err(MatcherError::DraftNotFoundError { draft_id: draft_id.to_string() });
        }

        let draft_dir = {
            let mut path = self.drafts_path.to_path_buf();
            path.push(draft_id);
            path
        };
        let mut draft_data: MatcherConfigDraftData =
            parse_node_config_from_file(&draft_dir).await?;

        if draft_data.user != user {
            warn!("User {user} tried overwriting a draft that is owned by {}.", draft_data.user);
            // todo: improve in NEPROD-1658
            return Err(MatcherError::ConfigurationError {
                message: format!(
                    "User [{}] cannot overwrite draft owned by [{}]",
                    user, draft_data.user
                ),
            });
        }

        draft_data.updated_ts_ms = Local::now().timestamp_millis();
        serialize_config_node_to_file(&draft_dir, &draft_data).await?;

        let draft_config_dir = {
            let mut path = draft_dir;
            path.push("config");
            path
        };
        atomic_deploy_config(&draft_config_dir, config).await?;
        Ok(())
    }

    async fn deploy_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError> {
        // ToDo: Do proper sanitation of the draft_id when multitenancy is added to avoid path-traversal vulnerabilities.
        if draft_id != DRAFT_ID {
            return Err(MatcherError::DraftNotFoundError { draft_id: draft_id.to_string() });
        }

        let draft = self.get_draft(draft_id).await?;
        atomic_deploy_config(&self.root_path, &draft.config).await?;
        Ok(draft.config)
    }

    async fn delete_draft(&self, draft_id: &str) -> Result<(), MatcherError> {
        // ToDo: Do proper sanitation of the draft_id when multitenancy is added to avoid path-traversal vulnerabilities.
        if draft_id != DRAFT_ID {
            return Err(MatcherError::DraftNotFoundError { draft_id: draft_id.to_string() });
        }

        info!("Deleting draft {}", draft_id);

        let draft_dir = {
            let mut path = self.drafts_path.to_path_buf();
            path.push(draft_id);
            path
        };

        if let Err(error) = tokio::fs::remove_dir_all(draft_dir).await {
            return Err(MatcherError::InternalSystemError {
                message: format!("Cannot delete draft [{}]: {:?}", draft_id, error),
            });
        }

        Ok(())
    }

    async fn draft_take_over(&self, draft_id: &str, user: String) -> Result<(), MatcherError> {
        // ToDo: Do proper sanitation of the draft_id when multitenancy is added to avoid path-traversal vulnerabilities.
        if draft_id != DRAFT_ID {
            return Err(MatcherError::DraftNotFoundError { draft_id: draft_id.to_string() });
        }

        let draft_dir = {
            let mut path = self.drafts_path.to_path_buf();
            path.push(draft_id);
            path
        };
        let mut draft_data: MatcherConfigDraftData =
            parse_node_config_from_file(&draft_dir).await?;
        info!("User {} is taking over draft {} from user {}", user, draft_id, draft_data.user);
        draft_data.user = user;
        serialize_config_node_to_file(&draft_dir, &draft_data).await?;
        Ok(())
    }

    async fn deploy_config(&self, config: &MatcherConfig) -> Result<MatcherConfig, MatcherError> {
        atomic_deploy_config(&self.root_path, config).await?;
        Ok(config.clone())
    }
}

async fn atomic_deploy_config(dir: &Path, config: &MatcherConfig) -> Result<(), MatcherError> {
    // Validate also regex and accessor, which the MatcherConfigValidator does not do.
    let _ = Matcher::build(config)?;
    let dir_canonical = match dir.canonicalize() {
        Ok(parent) => parent,
        Err(error) => {
            return Err(MatcherConfigError::DirIoError { path: dir.to_path_buf(), error }.into())
        }
    };
    let parent = dir_canonical.parent().unwrap_or(Path::new("/"));
    let tempdir =
        tempfile::tempdir_in(parent).map_err(|err| MatcherError::InternalSystemError {
            message: format!("Cannot create temporary directory. Err: {:?}", err),
        })?;

    serialize_config_node_to_file(tempdir.path(), &Version::default()).await?;
    match config {
        MatcherConfig::Filter { name, nodes, .. } if name == "root" => {
            deploy_child_nodes_to_dir(tempdir.path(), nodes).await?;
        }
        config => {
            // This branch should never be taken. If we read a config without root node by accident,
            // however, this will be the fallback.
            deploy_child_nodes_to_dir(tempdir.path(), &[config.clone()]).await?;
        }
    };

    if let Err(error) = tokio::fs::remove_dir_all(&dir_canonical).await {
        // todo: improve in NEPROD-1658
        return Err(MatcherError::InternalSystemError {
            message: format!(
                "Cannot remove config directory {}. {}",
                dir_canonical.display(),
                error
            ),
        });
    }

    // todo: If the machine looses power here, or the kernel panics, we can loose the whole or parts of the configuration.

    // Replace the directory inode. This is an atomic operation to overwrite the directory.
    if let Err(error) = tokio::fs::rename(tempdir.path(), &dir_canonical).await {
        return Err(DeploymentError::DirIo { error, path: dir_canonical.to_path_buf() }.into());
    }

    Ok(())
}

#[async_recursion::async_recursion]
async fn deploy_child_nodes_to_dir(
    path: &Path,
    nodes: &[MatcherConfig],
) -> Result<(), DeploymentError> {
    let mut futures_unordered = FuturesUnordered::new();
    for node in nodes {
        futures_unordered.push(deploy_child_node(path, node))
    }

    // Await the deployment concurrently to lessen the impact of all the syncs.
    while let Some(next) = futures_unordered.next().await {
        next?;
    }

    Ok(())
}

async fn deploy_child_node(path: &Path, node: &MatcherConfig) -> Result<(), DeploymentError> {
    let parent = create_sub_directory(path, node.get_name()).await?;
    match node {
        MatcherConfig::Filter { name, filter, nodes } => {
            deploy_filter_node(&parent, name, filter, nodes).await?;
        }
        MatcherConfig::Ruleset { name, rules } => {
            deploy_ruleset_node(&parent, name, rules).await?;
        }
        MatcherConfig::Iterator { name, iterator, nodes } => {
            deploy_iterator_node(&parent, name, iterator, nodes).await?
        }
    }

    Ok(())
}

async fn deploy_filter_node(
    dir: &Path,
    name: &str,
    filter: &Filter,
    nodes: &[MatcherConfig],
) -> Result<(), DeploymentError> {
    let config = MatcherConfigFilter {
        node_type: Default::default(),
        name: name.to_string(),
        filter: filter.clone(),
    };

    serialize_config_node_to_file(dir, &config).await?;
    deploy_child_nodes_to_dir(dir, nodes).await?;

    Ok(())
}

async fn deploy_iterator_node(
    dir: &Path,
    name: &str,
    iterator: &MatcherIterator,
    nodes: &[MatcherConfig],
) -> Result<(), DeploymentError> {
    let config = MatcherConfigIterator {
        node_type: Default::default(),
        name: name.to_string(),
        iterator: iterator.to_owned(),
    };

    serialize_config_node_to_file(dir, &config).await?;
    deploy_child_nodes_to_dir(dir, nodes).await?;

    Ok(())
}

async fn deploy_ruleset_node(
    dir: &Path,
    name: &str,
    rules: &[Rule],
) -> Result<(), DeploymentError> {
    let config = MatcherConfigRuleset { node_type: Default::default(), name: name.to_string() };
    serialize_config_node_to_file(dir, &config).await?;
    deploy_rules(dir, rules).await?;

    Ok(())
}

async fn deploy_rules(dir: &Path, rules: &[Rule]) -> Result<(), DeploymentError> {
    let rules_dir = create_sub_directory(dir, "rules").await?;
    for (index, rule) in rules.iter().enumerate() {
        let filename = format!("{:09}0_{}.json", index, rule.name);
        let config_file_path = {
            let mut path = rules_dir.clone();
            path.push(&filename);
            path
        };
        serialize_to_file(&config_file_path, rule).await?;
    }

    Ok(())
}

pub async fn serialize_config_node_to_file<T: Serialize + ConfigNodeDir>(
    dir: &Path,
    data: &T,
) -> Result<(), DeploymentError> {
    let node_file = {
        let mut path = dir.to_path_buf();
        path.push(T::config_type().filename());
        path
    };

    serialize_to_file(&node_file, data).await
}

async fn serialize_to_file<T: Serialize>(path: &Path, data: &T) -> Result<(), DeploymentError> {
    let bytes = match serde_json::to_vec_pretty(data) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(DeploymentError::Serialization {
                error,
                data_type: std::any::type_name::<T>(),
            })
        }
    };

    let mut file = match tokio::fs::File::options().write(true).create(true).open(path).await {
        Ok(file) => file,
        Err(error) => return Err(DeploymentError::DirIo { path: path.to_path_buf(), error }),
    };

    if let Err(error) = file.write_all(&bytes).await {
        return Err(DeploymentError::FileIo { path: path.to_path_buf(), error });
    }

    if let Err(error) = file.flush().await {
        return Err(DeploymentError::FileIo { path: path.to_path_buf(), error });
    }

    Ok(())
}

async fn create_sub_directory(dir: &Path, child_dir: &str) -> Result<PathBuf, DeploymentError> {
    let sub_dir_path = {
        let mut dir = dir.to_path_buf();
        dir.push(child_dir);
        dir
    };

    if let Err(error) = tokio::fs::create_dir(&sub_dir_path).await {
        return Err(DeploymentError::DirIo { path: sub_dir_path, error });
    }

    Ok(sub_dir_path)
}

async fn get_drafts(drafts_dir: &Path) -> Result<Vec<String>, MatcherConfigError> {
    debug!("Trying to read draft entries from {}", drafts_dir.display());
    if !tokio::fs::try_exists(drafts_dir).await.unwrap_or(false) {
        warn!("Draft directory {} does not exist.", drafts_dir.display());
        return Ok(vec![]);
    }

    let entries = gather_dir_entries(drafts_dir).await?;

    let mut drafts = vec![];
    for entry in entries {
        let entry_type = match entry.file_type().await {
            Ok(entry_type) => entry_type,
            Err(error) => return Err(MatcherConfigError::DirIoError { path: entry.path(), error }),
        };

        if !entry_type.is_dir() {
            warn!("Found a directory entry in the drafts directory, that is not itself a directory {}", entry.path().display());
            continue;
        }

        let entry_path = entry.path();
        let filename =
            entry_path.file_name().expect("Path comes from read_dir and is a valid path.");
        let Some(draft_name) = filename.to_str() else {
            error!("Found entry in the drafts directory with a non-utf8 name.");
            return Err(MatcherConfigError::FileNameError { path: entry_path });
        };

        debug!("Found draft with name {}.", draft_name);
        drafts.push(draft_name.to_owned());
    }

    Ok(drafts)
}

async fn get_draft_from_dir(draft_dir: &Path) -> Result<MatcherConfigDraft, MatcherConfigError> {
    debug!("Trying to load a draft from the directory {}", draft_dir.display());
    let draft_data = parse_node_config_from_file::<MatcherConfigDraftData>(draft_dir).await?;

    let draft_config_dir = {
        let mut path = draft_dir.to_path_buf();
        path.push("config");
        path
    };
    let draft_config = read_config_from_root_dir(&draft_config_dir).await?;

    Ok(MatcherConfigDraft { data: draft_data, config: draft_config })
}

async fn create_draft(
    processing_tree_dir: &Path,
    draft_dir: &Path,
    user: &str,
    draft_id: &str,
) -> Result<(), MatcherError> {
    info!("Creating a new draft {draft_id} for user {user}");

    let draft_config_dir = {
        let mut path = draft_dir.to_path_buf();
        path.push("config");
        path
    };

    let now = Local::now().timestamp_millis();
    let draft_data = MatcherConfigDraftData {
        created_ts_ms: now,
        updated_ts_ms: now,
        user: user.to_string(),
        draft_id: draft_id.to_string(),
    };

    if let Err(error) = tokio::fs::create_dir_all(draft_dir).await {
        return Err(MatcherError::InternalSystemError {
            message: format!("Cannot create draft directory: {:?}", error),
        });
    };
    serialize_config_node_to_file(draft_dir, &draft_data).await?;
    v1::fs::copy_and_override(processing_tree_dir, &draft_config_dir).await
}

#[cfg(test)]
mod tests {
    use crate::config::nodes::MatcherIterator;
    use crate::config::v1::fs::copy_recursive;
    use crate::config::v2::editor::{deploy_iterator_node, get_draft_from_dir, DRAFT_ID};
    use crate::config::v2::{
        parse_node_config_from_file, ConfigType, FsMatcherConfigManagerV2, MatcherConfigIterator,
    };
    use crate::config::{
        MatcherConfig, MatcherConfigDraftData, MatcherConfigEditor, MatcherConfigReader,
    };
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    const TEST_CONFIG_DIR: &str = "./test_resources/v2/test_config/";
    const TEST_DRAFT_DIR: &str = "./test_resources/v2/test_drafts/";

    #[tokio::test]
    async fn should_load_draft_data_from_file() {
        let draft_path = String::from(TEST_DRAFT_DIR) + "draft_001";
        let config = parse_node_config_from_file::<MatcherConfigDraftData>(Path::new(&draft_path))
            .await
            .unwrap();

        assert_eq!("root", config.user);
        assert_eq!("draft_001", config.draft_id);
    }

    #[tokio::test]
    async fn should_load_draft_from_config() {
        let draft_path = String::from(TEST_DRAFT_DIR) + "draft_001";
        let draft = get_draft_from_dir(Path::new(&draft_path)).await.unwrap();

        assert_eq!("root", draft.data.user);
        assert_eq!("draft_001", draft.data.draft_id);
    }

    #[tokio::test]
    async fn matcher_config_editor_should_get_drafts() {
        let config_manager =
            FsMatcherConfigManagerV2::new(Path::new(TEST_CONFIG_DIR), Path::new(TEST_DRAFT_DIR));
        let drafts = config_manager.get_drafts().await.unwrap();

        assert_eq!(1, drafts.len());
        assert_eq!("draft_001", drafts[0]);
    }

    #[tokio::test]
    async fn matcher_config_editor_should_get_draft() {
        let config_manager =
            FsMatcherConfigManagerV2::new(Path::new(TEST_CONFIG_DIR), Path::new(TEST_DRAFT_DIR));
        let draft = config_manager.get_draft("draft_001").await.unwrap();

        assert_eq!("root", draft.data.user);
        match draft.config {
            MatcherConfig::Filter { name, nodes, .. } => {
                assert_eq!("root", name);
                assert_eq!(1, nodes.len());
            }
            result => panic!("{:?}", result),
        }
    }

    #[tokio::test]
    async fn matcher_config_editor_should_create_draft() {
        // Arrange
        let temp_dir = TempDir::new().unwrap();
        let config_manager =
            FsMatcherConfigManagerV2::new(Path::new(TEST_CONFIG_DIR), temp_dir.path());

        // Act
        let draft_id = config_manager.create_draft(String::from("pippo")).await.unwrap();

        // Assert
        let draft = config_manager.get_draft(&draft_id).await.unwrap();
        assert_eq!("pippo", draft.data.user);
        match draft.config {
            MatcherConfig::Filter { name, nodes, .. } => {
                assert_eq!("root", name);
                assert_eq!(3, nodes.len());
            }
            result => panic!("{:?}", result),
        }
    }

    #[tokio::test]
    async fn matcher_config_editor_should_update_draft() {
        // Arrange
        let temp_dir = TempDir::new().unwrap();
        let config_manager =
            FsMatcherConfigManagerV2::new(Path::new(TEST_CONFIG_DIR), temp_dir.path());
        let draft_id = config_manager.create_draft(String::from("pippo")).await.unwrap();

        let new_draft_path = {
            let mut path = PathBuf::from(TEST_DRAFT_DIR);
            path.push("draft_001");
            path
        };

        let new_draft = get_draft_from_dir(&new_draft_path).await.unwrap();

        // Act
        config_manager
            .update_draft(&draft_id, String::from("pippo"), &new_draft.config)
            .await
            .unwrap();

        // Assert
        let draft = config_manager.get_draft(&draft_id).await.unwrap();
        assert_eq!("pippo", draft.data.user);
        assert!(draft.data.created_ts_ms < draft.data.updated_ts_ms);
        match draft.config {
            MatcherConfig::Filter { name, nodes, .. } => {
                assert_eq!("root", name);
                assert_eq!(1, nodes.len());
            }
            result => panic!("{:?}", result),
        }
    }

    #[tokio::test]
    async fn matcher_config_editor_should_delete_draft() {
        // Arrange
        let temp_dir = TempDir::new().unwrap();
        let config_manager =
            FsMatcherConfigManagerV2::new(Path::new(TEST_CONFIG_DIR), temp_dir.path());
        let draft_id = config_manager.create_draft(String::from("pippo")).await.unwrap();

        let draft_config_path = {
            let mut path = config_manager.drafts_path.to_path_buf();
            path.push(&draft_id);
            path.push(ConfigType::Draft.filename());
            path
        };

        // Assert the draft exists, so it fails if no draft was ever present.
        assert!(draft_config_path.exists());

        // Act
        config_manager.delete_draft(&draft_id).await.unwrap();

        // Assert
        assert!(!draft_config_path.exists());
    }

    #[tokio::test]
    async fn matcher_config_editor_should_deploy_draft() {
        // Arrange
        let temp_dir = TempDir::new().unwrap();
        let draft_temp_dir = {
            let mut path = temp_dir.path().to_path_buf();
            path.push("drafts");
            path
        };
        let config_temp_dir = {
            let mut path = temp_dir.path().to_path_buf();
            path.push("rules.d");
            path
        };

        let config_manager =
            FsMatcherConfigManagerV2::new(config_temp_dir.as_path(), draft_temp_dir.as_path());
        copy_recursive(PathBuf::from(TEST_CONFIG_DIR), config_temp_dir.clone()).await.unwrap();
        copy_recursive(PathBuf::from(TEST_DRAFT_DIR), draft_temp_dir.clone()).await.unwrap();

        // Assert that the old config is present
        let config = config_manager.get_config().await.unwrap();
        match config {
            MatcherConfig::Filter { name, nodes, .. } => {
                assert_eq!("root", name);
                assert_eq!(3, nodes.len());
            }
            result => panic!("{:?}", result),
        }

        // Act
        config_manager.deploy_draft(DRAFT_ID).await.unwrap();

        // Assert
        let config = config_manager.get_config().await.unwrap();
        match config {
            MatcherConfig::Filter { name, nodes, .. } => {
                assert_eq!("root", name);
                assert_eq!(1, nodes.len());
            }
            result => panic!("{:?}", result),
        }
    }

    #[tokio::test]
    async fn matcher_config_editor_should_take_over_draft() {
        // Arrange
        let temp_dir = TempDir::new().unwrap();
        let config_manager =
            FsMatcherConfigManagerV2::new(Path::new(TEST_CONFIG_DIR), temp_dir.path());
        let draft_id = config_manager.create_draft("pippo".to_string()).await.unwrap();
        let config_old = config_manager.get_draft(&draft_id).await.unwrap();

        // Act
        config_manager.draft_take_over(&draft_id, "root".to_string()).await.unwrap();

        // Assert
        let config = config_manager.get_draft(&draft_id).await.unwrap();
        assert_eq!(config_old.data.created_ts_ms, config.data.created_ts_ms);
        assert_eq!(config_old.data.updated_ts_ms, config.data.updated_ts_ms);
        assert_eq!(config_old.data.draft_id, config.data.draft_id);
        assert_eq!("pippo", config_old.data.user);
        assert_eq!("root", config.data.user);
    }

    #[tokio::test]
    async fn should_deploy_and_load_iterator_node() {
        let temp_dir = TempDir::new().unwrap();
        let config = MatcherConfig::Ruleset { name: "ruleset".to_string(), rules: vec![] };

        deploy_iterator_node(
            temp_dir.path(),
            "master_iterator",
            &MatcherIterator {
                description: "".to_string(),
                active: true,
                target: "${event.payload.alerts}.to_string()".to_string(),
            },
            &[config],
        )
        .await
        .unwrap();

        let loaded: MatcherConfigIterator =
            parse_node_config_from_file(temp_dir.path()).await.unwrap();

        assert_eq!("master_iterator", loaded.name);
    }
}
