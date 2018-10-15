use std::collections::HashMap;

pub struct Event {
    pub event_type: String,
    pub created_ts: u64,
    pub payload: HashMap<String, String>,
}
