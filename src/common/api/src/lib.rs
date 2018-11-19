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
    pub created_ts: String,
    pub payload: HashMap<String, String>,
}

impl Event {
    pub fn new<S: Into<String>>(event_type: S) -> Event {
        Event::new_with_payload(event_type, HashMap::new())
    }

    pub fn new_with_payload<S: Into<String>>(event_type: S, payload: HashMap<String, String>) -> Event {
        let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
        let created_ts: String = dt.to_rfc3339();
        Event { event_type: event_type.into(), created_ts, payload }
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
    pub fn new<S: Into<String>>(id: S) -> Action {
        Action { id: id.into(), payload: HashMap::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Text(String),
    // Array(Vec<Value>),
    // Map(HashMap<String, Value>),
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::prelude::*;

    #[test]
    fn created_ts_should_be_iso_8601() {
        let event = Event::new("");

        let created_ts = event.created_ts;
        println!("created_ts: [{}]", created_ts);

        let dt = DateTime::parse_from_rfc3339(&created_ts);
        assert!(dt.is_ok());

    }
}