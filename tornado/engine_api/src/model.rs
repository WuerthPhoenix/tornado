use crate::auth::auth_v2::AuthServiceV2;
use crate::auth::AuthService;
use serde::{Deserialize, Serialize};
use tornado_engine_matcher::config::MatcherConfig;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "version")]
pub enum ExportVersionedMatcherConfig {
    #[serde(rename = "1.0")]
    V1(MatcherConfig),
    #[serde(rename = "1.1")]
    V1_1(MatcherConfig),
}

pub struct ApiData<T> {
    pub auth: AuthService,
    pub api: T,
}

pub struct ApiDataV2<T> {
    pub auth: AuthServiceV2,
    pub api: T,
}
