use crate::ElasticsearchAuthentication;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct ElasticsearchConfig {
    pub default_auth: Option<ElasticsearchAuthentication>,
}
