use crate::config::{MatcherConfig, MatcherConfigReader};
use crate::error::MatcherError;
use rs_consul::{Config, ConsistencyMode, Consul, ReadKeyRequest};

pub mod editor;

pub const ROOT_NODE_NAME: &str = "root";
const CONSUL_CONFIG_KEY: &str = "/tornado/config";
const CONSUL_DRAFT_KEY: &str = "/tornado/draft";

pub struct ConsulMatcherConfigManager {
    client: Consul,
    base_path: String,
}

impl ConsulMatcherConfigManager {
    pub async fn new(base_path: String) -> Result<ConsulMatcherConfigManager, MatcherError> {
        let config = Config { address: "172.17.0.3:8500".to_string(), token: None };
        let client = Consul::new(config);
        Ok(Self { base_path, client })
    }

    fn config_path(&self) -> String {
        format!("{}{}", self.base_path, CONSUL_CONFIG_KEY)
    }

    fn draft_path(&self) -> String {
        format!("{}{}", self.base_path, CONSUL_DRAFT_KEY)
    }
}

#[async_trait::async_trait(?Send)]
impl MatcherConfigReader for ConsulMatcherConfigManager {
    async fn get_config(&self) -> Result<MatcherConfig, MatcherError> {
        let read_key_request = ReadKeyRequest {
            key: &self.config_path(),
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
                message: format!("Error while fetching the config. Err: {}", err),
            }
        })?;
        match response_keys.into_iter().next() {
            None => Err(MatcherError::InternalSystemError {
                message: format!("No config found for key {}.", &self.config_path()),
            }),
            Some(response) => match response.value {
                None => Err(MatcherError::InternalSystemError {
                    message: format!("No value found for key {}.", &self.config_path()),
                }),
                Some(value) => {
                    let value =
                        base64::decode(value).map_err(|err| MatcherError::InternalSystemError {
                            message: format!("Could not base64 decode config. Err: {}", err),
                        })?;
                    serde_json::from_slice(&value).map_err(|err| {
                        MatcherError::InternalSystemError {
                            message: format!("Could not deserialize config. Err: {}", err),
                        }
                    })
                }
            },
        }
    }
}
