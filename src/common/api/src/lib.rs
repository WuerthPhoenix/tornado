use std::collections::HashMap;

/// An Event is correlated with an incoming action, incident, situation or whatever kind of episode that could is meaningful for the system.
/// It is produced by the collectors and sent to the Tornado Engine to be processed.
pub struct Event {
    pub event_type: String,
    pub created_ts: u64,
    pub payload: HashMap<String, String>,
}
