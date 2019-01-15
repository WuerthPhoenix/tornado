use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct JMESPathEventCollectorConfig {
    pub event_type: String,
    pub payload: HashMap<String, String>,
}
