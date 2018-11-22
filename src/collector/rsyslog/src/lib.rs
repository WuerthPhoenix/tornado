extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tornado_collector_common;
extern crate tornado_common_api;

use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::{Event};


pub mod model;

pub const EVENT_TYPE: &str = "syslog";

#[derive(Default)]
pub struct RsyslogCollector {}

impl RsyslogCollector {
    pub fn new() -> RsyslogCollector {
        Default::default()
    }
}

impl Collector<model::Json> for RsyslogCollector {
    fn to_event(&self, json: model::Json) -> Result<Event, CollectorError> {
        Ok(Event::new_with_payload(EVENT_TYPE.to_owned(), model::to_payload(json)))
    }
}

#[cfg(test)]
extern crate serde_json;

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;

    #[test]
    fn should_produce_event_from_rsyslog_json_input() {
        // Arrange
        let rsyslog_filename = "./test_resources/rsyslog_01_input.json";
        let rsyslog_string =
            fs::read_to_string(rsyslog_filename).expect(&format!("Unable to open the file [{}]", rsyslog_filename));
        let rsyslog_json =serde_json::from_str::<model::Json>(&rsyslog_string).expect("should parse the rsyslog json");

        let expected_event_filename = "./test_resources/rsyslog_01_output.json";
        let expected_event_string =
            fs::read_to_string(expected_event_filename).expect(&format!("Unable to open the file [{}]", expected_event_filename));
        let mut expected_event =serde_json::from_str::<Event>(&expected_event_string).expect("should parse the event");

        let collector = RsyslogCollector::new();

        // Act
        let event = collector.to_event(rsyslog_json).unwrap();

        // Assert
        assert_eq!(EVENT_TYPE, &event.event_type);
        assert_eq!("2018-11-01T23:59:59+01:00", event.payload.get("@timestamp").unwrap());

        expected_event.created_ts = event.created_ts.clone();
        assert_eq!(expected_event, event);
    }
}

