extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;

/// An Event is correlated with an incoming episode, incident, situation or whatever kind of case that could meaningful for the system.
/// It is produced by the collectors and sent to the Tornado Engine to be processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_type: String,
    pub created_ts: u64,
    pub payload: HashMap<String, String>,
}

/// Action is produced when an Event matches a specific Rule.
/// It is produced by the Tornado Engine and sent to the Executors to be resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    id: String,
    payload: HashMap<String, String>,
}
