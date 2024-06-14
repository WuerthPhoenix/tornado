use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::config::v1::fs::FsMatcherConfigManager;
use crate::config::v2::error::DeploymentError;
use crate::config::v2::MatcherConfigError::UnexpectedFile;
use crate::config::v2::{
    gather_dir_entries, parse_from_file, parse_node_config_from_file, read_config_from_root_dir,
    ConfigNodeDir, ConfigType, FsMatcherConfigManagerV2, MatcherConfigError, MatcherConfigFilter,
    MatcherConfigRuleset, Version,
};
use crate::config::{
    MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData, MatcherConfigEditor,
};
use crate::error::MatcherError;
use crate::matcher::Matcher;
use chrono::Local;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

const DRAFT_ID: &str = "draft_001";

#[async_trait::async_trait(?Send)]
impl MatcherConfigEditor for FsMatcherConfigManagerV2<'_> {
    async fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
        Ok(get_drafts(self.drafts_path).await?)
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

        create_draft(self.root_path, &draft_path, &user, DRAFT_ID).await?;
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
            let mut path = self.drafts_path.to_path_buf();
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
        atomic_deploy_config(self.root_path, &draft.config).await?;
        self.delete_draft(draft_id).await?;
        Ok(draft.config)
    }

    async fn delete_draft(&self, draft_id: &str) -> Result<(), MatcherError> {
        // ToDo: Do proper sanitation of the draft_id when multitenancy is added to avoid path-traversal vulnerabilities.
        if draft_id != DRAFT_ID {
            return Err(MatcherError::DraftNotFoundError { draft_id: draft_id.to_string() });
        }

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
        draft_data.user = user;
        serialize_config_node_to_file(&draft_dir, &draft_data).await?;
        Ok(())
    }

    async fn deploy_config(&self, config: &MatcherConfig) -> Result<MatcherConfig, MatcherError> {
        atomic_deploy_config(self.root_path, config).await?;
        Ok(config.clone())
    }
}

async fn atomic_deploy_config(dir: &Path, config: &MatcherConfig) -> Result<(), MatcherError> {
    // Validate also regex and accessor, which the MatcherConfigValidator does not do.
    let _ = Matcher::build(config)?;
    let tempdir = tempfile::tempdir().map_err(|err| MatcherError::InternalSystemError {
        message: format!("Cannot create temporary directory. Err: {:?}", err),
    })?;

    serialize_config_node_to_file(&dir, &Version::default()).await?;
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

    // Replace the directory inode. This is an atomic operation to overwrite the directory.
    if let Err(error) = tokio::fs::rename(tempdir.path(), dir).await {
        return Err(DeploymentError::DirIo { error, path: dir.to_path_buf() }.into());
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

    serialize_config_node_to_file(&dir, &config).await?;
    deploy_child_nodes_to_dir(dir, nodes).await?;

    Ok(())
}

async fn deploy_ruleset_node(
    dir: &Path,
    name: &str,
    rules: &[Rule],
) -> Result<(), DeploymentError> {
    let config = MatcherConfigRuleset { node_type: Default::default(), name: name.to_string() };
    serialize_config_node_to_file(&dir, &config).await?;
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

async fn serialize_config_node_to_file<T: Serialize + ConfigNodeDir>(
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

    if let Err(error) = file.sync_all().await {
        return Err(DeploymentError::FileIo { path: path.to_path_buf(), error });
    }

    let parent = path.parent().unwrap_or(&Path::new("/"));
    let parent_dir = match tokio::fs::File::open(parent).await {
        Ok(parent_dir) => parent_dir,
        Err(error) => return Err(DeploymentError::FileIo { path: path.to_path_buf(), error }),
    };

    if let Err(error) = parent_dir.sync_all().await {
        return Err(DeploymentError::DirIo { path: path.to_path_buf(), error });
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

    let sub_dir = match tokio::fs::File::open(&sub_dir_path).await {
        Ok(parent_dir) => parent_dir,
        Err(error) => {
            return Err(DeploymentError::FileIo { path: sub_dir_path.to_path_buf(), error })
        }
    };
    if let Err(error) = sub_dir.sync_all().await {
        return Err(DeploymentError::DirIo { path: sub_dir_path.to_path_buf(), error });
    }

    let parent_dir = match tokio::fs::File::open(dir).await {
        Ok(parent_dir) => parent_dir,
        Err(error) => return Err(DeploymentError::FileIo { path: dir.to_path_buf(), error }),
    };
    if let Err(error) = parent_dir.sync_all().await {
        return Err(DeploymentError::DirIo { path: dir.to_path_buf(), error });
    }

    Ok(sub_dir_path)
}

async fn get_drafts(drafts_dir: &Path) -> Result<Vec<String>, MatcherConfigError> {
    if !tokio::fs::try_exists(drafts_dir).await.unwrap_or(false) {
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
            return Err(UnexpectedFile { path: entry.path(), config_type: ConfigType::Root });
        }

        let entry_path = entry.path();
        let filename =
            entry_path.file_name().expect("Path comes from read_dir and is a valid path.");
        let Some(draft_name) = filename.to_str() else {
            return Err(MatcherConfigError::FileNameError { path: entry_path });
        };
        drafts.push(draft_name.to_owned());
    }

    Ok(drafts)
}

async fn get_draft_from_dir(draft_dir: &Path) -> Result<MatcherConfigDraft, MatcherConfigError> {
    let draft_data = parse_node_config_from_file::<MatcherConfigDraftData>(&draft_dir).await?;

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

    if let Err(error) = tokio::fs::create_dir(draft_dir).await {
        return Err(MatcherError::InternalSystemError {
            message: format!("Cannot create draft directory: {:?}", error),
        });
    };
    serialize_config_node_to_file(&draft_dir, &draft_data).await?;
    FsMatcherConfigManager::copy_and_override(processing_tree_dir, &draft_config_dir).await
}

#[test]
fn test() {
    std::fs::File::open(".").unwrap().sync_all().unwrap();
}
