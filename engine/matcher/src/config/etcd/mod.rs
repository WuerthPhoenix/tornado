use crate::config::{MatcherConfig, MatcherConfigReader};
use crate::error::MatcherError;
use etcd_client::Client;
use log::*;


pub mod editor;

pub const ROOT_NODE_NAME: &str = "root";
pub const ECTD_CONFIG_KEY: &str = "/tornado/config";
pub const ECTD_DRAF_KEY: &str = "/tornado/draft";

pub struct EtcdMatcherConfigManager {
    client: Client,
}

impl EtcdMatcherConfigManager {
    pub async fn new() -> Result<EtcdMatcherConfigManager, MatcherError> {
        let client = Client::connect(["127.0.0.1:2379"], None).await
        .map_err(|err| MatcherError::ConfigurationError {
            message: format!("Cannot connect to ETCD. Err: {:?}", err)
        })?;
        Ok(Self {
            client
        })
    }
}

#[async_trait::async_trait(?Send)]
impl MatcherConfigReader for EtcdMatcherConfigManager {
    async fn get_config(&self) -> Result<MatcherConfig, MatcherError> {
        debug!("Fetch configuration from Etcd");
        let mut client = self.client.kv_client();
        let result = client.get(ECTD_CONFIG_KEY, None).await
        .map_err(|err| MatcherError::ConfigurationError {
            message: format!("Cannot GET value from ETCD. Err: {:?}", err)
        })?;

        if let Some(config) = result.kvs().iter().next() {
            serde_json::from_slice(config.value()).map_err(|err| MatcherError::ConfigurationError {
                message: format!("Cannot deserialize config get from ETCD. Err: {:?}", err)
            })
        } else {
            Err(MatcherError::ConfigurationError {
                message: "Configuration not found in ETCD.".to_owned()
            })
        }
    }
}
