use crate::config::filter::Filter;
use crate::config::{
    Defaultable, MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData, MatcherConfigEditor,
    MatcherConfigReader,
};
use crate::error::MatcherError;
use crate::validator::MatcherConfigValidator;
use chrono::Local;
use log::*;

use super::{EtcdMatcherConfigManager};
use crate::config::etcd::{ROOT_NODE_NAME};

pub const DRAFT_ID: &str = "draft_001";

#[async_trait::async_trait(?Send)]
impl MatcherConfigEditor for EtcdMatcherConfigManager {
    async fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
        debug!("Get drafts from Etcd");

        let mut client = self.client.kv_client();
        let result = client.get(self.draft_path().as_str(), None).await
        .map_err(|err| MatcherError::ConfigurationError {
            message: format!("Cannot GET draft from ETCD. Err: {:?}", err)
        })?;
        
        let mut draft_ids = vec![];
        for kv in result.kvs().iter() {
            let draft: MatcherConfigDraft = serde_json::from_slice(kv.value()).map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot deserialize draft get from ETCD. Err: {:?}", err)
            })?;
            draft_ids.push(draft.data.draft_id);
        }

        Ok(draft_ids)
    }

    async fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError> {
        debug!("Get draft with id {}", draft_id);

        let mut client = self.client.kv_client();
        let result = client.get(self.draft_path().as_str(), None).await
        .map_err(|err| MatcherError::ConfigurationError {
            message: format!("Cannot GET draft from ETCD. Err: {:?}", err)
        })?;
        
        if let Some(config) = result.kvs().iter().next() {
            serde_json::from_slice(config.value()).map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot deserialize draft get from ETCD. Err: {:?}", err)
            })
        } else {
            Err(MatcherError::ConfigurationError {
                message: "Draft not found in ETCD.".to_owned()
            })
        }
    }

    async fn create_draft(&self, user: String) -> Result<String, MatcherError> {
        info!("Create new draft");

        let draft_id = DRAFT_ID.to_owned();

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

        let draft = MatcherConfigDraft {
            config: current_config,
            data: MatcherConfigDraftData {
                user,
                updated_ts_ms: current_ts_ms,
                created_ts_ms: current_ts_ms,
                draft_id: draft_id.clone(),
            }
        };

        let draft_json_string = serde_json::to_vec(&draft)
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot serialize draft to JSON. Err: {:?}", err)
            })?;
        let mut client = self.client.kv_client();
        let _result = client.put(self.draft_path().as_str(), draft_json_string, None).await
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot GET draft from ETCD. Err: {:?}", err)
            })?;

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

        let mut current_draft = self.get_draft(DRAFT_ID).await?;
        current_draft.data.user = user;
        current_draft.data.updated_ts_ms = current_ts_ms();
        current_draft.config = config.clone();

        let draft_json_string = serde_json::to_vec(&current_draft)
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot serialize draft to JSON. Err: {:?}", err)
            })?;
        let mut client = self.client.kv_client();
        let _result = client.put(self.draft_path().as_str(), draft_json_string, None).await
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot PUT draft to ETCD. Err: {:?}", err)
            })?;

        Ok(())
    }

    async fn deploy_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError> {
        info!("Deploy draft with id {}", draft_id);
        let draft_id = DRAFT_ID;
        let draft = self.get_draft(draft_id).await?;
        self.deploy_config(&draft.config).await
    }

    async fn delete_draft(&self, draft_id: &str) -> Result<(), MatcherError> {
        info!("Delete draft with id {}", draft_id);

        let mut client = self.client.kv_client();
        let _result = client.delete(self.draft_path().as_str(), None).await
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot DELETE draft from ETCD. Err: {:?}", err)
            })?;

        Ok(())
    }

    async fn draft_take_over(&self, draft_id: &str, user: String) -> Result<(), MatcherError> {
        info!("User [{}] asks to take over draft with id {}", user, draft_id);

        let mut current_draft = self.get_draft(DRAFT_ID).await?;
        current_draft.data.user = user;

        let draft_json_string = serde_json::to_vec(&current_draft)
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot serialize draft to JSON. Err: {:?}", err)
            })?;
        let mut client = self.client.kv_client();
        let _result = client.put(self.draft_path().as_str(), draft_json_string, None).await
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot PUT draft to ETCD. Err: {:?}", err)
            })?;
        Ok(())
    }

    async fn deploy_config(&self, config: &MatcherConfig) -> Result<MatcherConfig, MatcherError> {
        info!("Deploy new configuration");

        MatcherConfigValidator::new().validate(config)?;

        let draft_json_string = serde_json::to_vec(config)
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot serialize draft to JSON. Err: {:?}", err)
            })?;
        let mut client = self.client.kv_client();
        let _result = client.put(self.config_path().as_str(), draft_json_string, None).await
            .map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot PUT config to ETCD. Err: {:?}", err)
            })?;
        Ok(config.clone())
    }
}

pub fn current_ts_ms() -> i64 {
    Local::now().timestamp_millis()
}
