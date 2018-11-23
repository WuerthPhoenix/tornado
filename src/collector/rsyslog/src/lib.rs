extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tornado_collector_common;
extern crate tornado_common_api;

use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::Event;

pub mod model;

pub const EVENT_TYPE: &str = "syslog";

#[derive(Default)]
pub struct RsyslogCollector {}

impl RsyslogCollector {
    pub fn new() -> RsyslogCollector {
        Default::default()
    }
}

impl<'a> Collector<&'a str> for RsyslogCollector {
    fn to_event(&self, json_str: &'a str) -> Result<Event, CollectorError> {
        let json = serde_json::from_str::<model::Json>(json_str)
            .map_err(|e| CollectorError::JsonParsingError { message: format!("{}", e) })?;
        Ok(Event::new_with_payload(EVENT_TYPE.to_owned(), model::to_payload(json)))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;

    #[test]
    fn should_produce_event_from_rsyslog_json_input() {
        // Arrange
        let rsyslog_filename = "./test_resources/rsyslog_01_input.json";
        let rsyslog_string = fs::read_to_string(rsyslog_filename)
            .expect(&format!("Unable to open the file [{}]", rsyslog_filename));

        let expected_event_filename = "./test_resources/rsyslog_01_output.json";
        let expected_event_string = fs::read_to_string(expected_event_filename)
            .expect(&format!("Unable to open the file [{}]", expected_event_filename));
        let mut expected_event =
            serde_json::from_str::<Event>(&expected_event_string).expect("should parse the event");

        let collector = RsyslogCollector::new();

        // Act
        let event = collector.to_event(&rsyslog_string).unwrap();

        // Assert
        assert_eq!(EVENT_TYPE, &event.event_type);
        assert_eq!("2018-11-01T23:59:59+01:00", event.payload.get("@timestamp").unwrap());

        expected_event.created_ts = event.created_ts.clone();
        assert_eq!(expected_event, event);
    }
}
