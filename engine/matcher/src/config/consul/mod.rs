use crate::config::{MatcherConfig, MatcherConfigReader};
use crate::error::MatcherError;
use rs_consul::{Config, Consul, ReadKeyRequest};

pub mod editor;

pub const ROOT_NODE_NAME: &str = "root";
const CONSUL_CONFIG_KEY: &str = "tornado/config";
const CONSUL_DRAFT_KEY: &str = "tornado/draft";

pub struct ConsulMatcherConfigManager {
    client: Consul,
    base_path: String,
}

impl ConsulMatcherConfigManager {
    pub async fn new(base_path: String) -> Result<ConsulMatcherConfigManager, MatcherError> {
        let config = Config { address: "http://localhost:8500".to_string(), token: None };
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
    // TODO: when there is no config in consul, it replies with a 404 error, so this returns Err
    async fn get_config(&self) -> Result<MatcherConfig, MatcherError> {
        let read_key_request = ReadKeyRequest {
            key: &self.config_path(),
            .. Default::default()
        };
        let response_keys = self.client.read_key(read_key_request).await.map_err(|err| {
            MatcherError::InternalSystemError {
                message: format!("Error while fetching the config. Consul key: {}. Err: {}", &self.config_path(), err),
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
                    serde_json::from_str(&value).map_err(|err| {
                        MatcherError::InternalSystemError {
                            message: format!("Could not deserialize config. Err: {}", err),
                        }
                    })
                }
            },
        }
    }
}
