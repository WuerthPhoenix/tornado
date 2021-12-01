use crate::config::consul::ConsulMatcherConfigManager;
use crate::config::filter::Filter;
use crate::config::fs::ROOT_NODE_NAME;
use crate::config::{
    Defaultable, MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData, MatcherConfigEditor,
    MatcherConfigReader,
};
use crate::error::MatcherError;
use crate::validator::MatcherConfigValidator;
use chrono::Local;
use log::*;
use rs_consul::{ConsistencyMode, CreateOrUpdateKeyRequest, DeleteKeyRequest, ReadKeyRequest};
use serde::Serialize;

const DRAFT_ID: &str = "draft_001";

#[async_trait::async_trait(?Send)]
impl MatcherConfigEditor for ConsulMatcherConfigManager {
    async fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
        let read_key_request = ReadKeyRequest {
            key: &self.draft_path(),
            namespace: "",
            datacenter: "",
            recurse: true,
            separator: "",
            consistency: ConsistencyMode::Default,
            index: None,
            wait: Default::default(),
        };
        let response_keys = self.client.read_key(read_key_request).await.map_err(|err| {
            MatcherError::InternalSystemError {
                message: format!("Error while fetching the drafts. Err: {}", err),
            }
        })?;
        Ok(response_keys.into_iter().map(|response| response.key).collect())
    }

    // TODO: maybe draft_id should only be the consul key suffix instead of full key
    async fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError> {
        let read_key_request = ReadKeyRequest {
            key: &self.draft_path(),
            namespace: "",
            datacenter: "",
            recurse: false,
            separator: "",
            consistency: ConsistencyMode::Default,
            index: None,
            wait: Default::default(),
        };
        let response_keys = self.client.read_key(read_key_request).await.map_err(|err| {
            MatcherError::InternalSystemError {
                message: format!(
                    "Error while fetching the draft with id {}. Err: {}",
                    draft_id, err
                ),
            }
        })?;
        match response_keys.into_iter().next() {
            None => Err(MatcherError::InternalSystemError {
                message: format!("No draft found for id {}.", draft_id),
            }),
            Some(response) => match response.value {
                None => Err(MatcherError::InternalSystemError {
                    message: format!("No draft found for id {}.", draft_id),
                }),
                Some(value) => {
                    let value =
                        base64::decode(value).map_err(|err| MatcherError::InternalSystemError {
                            message: format!("Could not base64 decode draft. Err: {}", err),
                        })?;
                    serde_json::from_slice(&value).map_err(|err| {
                        MatcherError::InternalSystemError {
                            message: format!("Could not deserialize draft. Err: {}", err),
                        }
                    })
                }
            },
        }
    }

    async fn create_draft(&self, user: String) -> Result<String, MatcherError> {
        let current_ts_ms = current_ts_ms();
        let draft_id = format!("{}/{}", &self.draft_path(), DRAFT_ID);

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
            },
        };

        self.put_kv_pair(&draft_id, &draft).await?;
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

        self.put_kv_pair(draft_id, &current_draft).await
    }

    async fn deploy_draft(&self, draft_id: &str) -> Result<MatcherConfig, MatcherError> {
        info!("Deploy draft with id {}", draft_id);
        let draft_id = DRAFT_ID;
        let draft = self.get_draft(draft_id).await?;
        self.deploy_config(&draft.config).await
    }

    async fn delete_draft(&self, draft_id: &str) -> Result<(), MatcherError> {
        info!("Delete draft with id {}", draft_id);
        let delete_draft_request = DeleteKeyRequest {
            key: draft_id,
            datacenter: "",
            recurse: false,
            check_and_set: 0,
            namespace: "",
        };

        let deleted = self.client.delete_key(delete_draft_request).await.map_err(|err| {
            MatcherError::InternalSystemError {
                message: format!("Could not delete draft {} from Consul. Err: {}", draft_id, err),
            }
        })?;

        match deleted {
            true => Ok(()),
            false => Err(MatcherError::InternalSystemError {
                message: format!("Could not delete draft {} from Consul.", draft_id),
            }),
        }
    }

    async fn draft_take_over(&self, draft_id: &str, user: String) -> Result<(), MatcherError> {
        info!("User [{}] asks to take over draft with id {}", user, draft_id);

        let mut current_draft = self.get_draft(DRAFT_ID).await?;
        current_draft.data.user = user;

        self.put_kv_pair(draft_id, &current_draft).await
    }

    async fn deploy_config(&self, config: &MatcherConfig) -> Result<MatcherConfig, MatcherError> {
        info!("Deploy new configuration");

        MatcherConfigValidator::new().validate(config)?;

        self.put_kv_pair(&self.config_path(), config).await?;
        Ok(config.clone())
    }
}

pub fn current_ts_ms() -> i64 {
    Local::now().timestamp_millis()
}

impl ConsulMatcherConfigManager {
    async fn put_kv_pair<T: Serialize>(&self, key: &str, value: &T) -> Result<(), MatcherError> {
        let config_json_string =
            serde_json::to_vec(value).map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot serialize value to JSON. Err: {:?}", err),
            })?;
        let update_config_request = CreateOrUpdateKeyRequest {
            key,
            namespace: "",
            datacenter: "",
            flags: 0,
            check_and_set: 0,
            acquire: "",
            release: "",
        };
        let (updated, _index) = self
            .client
            .create_or_update_key(update_config_request, config_json_string)
            .await
            .map_err(|err| MatcherError::InternalSystemError {
                message: format!("Could not update key value pair. Key: {}. Err: {}", key, err),
            })?;
        match updated {
            true => {
                debug!("Key value pair updated for key: {}", key);
                Ok(())
            }
            false => Err(MatcherError::InternalSystemError {
                message: format!("Key value pair could not be pushed to Counsul. Key: {}", key),
            }),
        }
    }
}
