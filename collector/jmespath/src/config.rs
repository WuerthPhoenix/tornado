use serde::{Deserialize, Serialize};
use tornado_common_api::Payload;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct JMESPathEventCollectorConfig {
    pub event_type: String,
    pub payload: Payload,
}
