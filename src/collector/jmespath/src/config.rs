use serde_derive::{Deserialize, Serialize};
use tornado_common_api::Payload;

#[derive(Deserialize, Serialize, Clone)]
pub struct JMESPathEventCollectorConfig {
    pub event_type: String,
    pub payload: Payload,
}
