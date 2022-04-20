use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tornado_common_api::Value;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct JMESPathEventCollectorConfig {
    pub event_type: String,
    pub payload: HashMap<String, Value>,
}
