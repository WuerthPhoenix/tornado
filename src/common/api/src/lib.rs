extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate chrono;

use chrono::prelude::Local;
use std::collections::HashMap;

/// An Event is correlated with an incoming episode, incident, situation or whatever kind of case that could meaningful for the system.
/// It is produced by the collectors and sent to the Tornado Engine to be processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_type: String,
    pub created_ts: u64,
    pub payload: HashMap<String, String>,
}

impl Event {
    pub fn new(event_type: String) -> Event {
        let dt = Local::now();
        let created_ts = dt.timestamp_millis() as u64;
        Event { event_type, created_ts, payload: HashMap::new() }
    }
}

/// Action is produced when an Event matches a specific Rule.
/// It is produced by the Tornado Engine and sent to the Executors to be resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub payload: HashMap<String, String>,
}

impl Action {
    pub fn new(id: String) -> Action {
        Action { id, payload: HashMap::new() }
    }
}
