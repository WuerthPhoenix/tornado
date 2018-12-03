extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate tornado_collector_common;
extern crate tornado_common_api;

use regex::Regex;
use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::{Event, Payload, Value};

const ADDRESS_PARSE_REGEX: &str = r#"^([^:]+):\s\[([^\]]+)\]:([0-9]+)->\[([^\]]+)\]"#;

/// A collector that receives snmptrad input messages formatted as JSON and create the related Event struct.
pub struct SnmptradpCollector {
    address_regex: Regex,
}

impl Default for SnmptradpCollector {
    fn default() -> Self {
        SnmptradpCollector {
            address_regex: Regex::new(ADDRESS_PARSE_REGEX)
                .expect("SnmptradpCollector regex should be valid"),
        }
    }
}

impl SnmptradpCollector {
    pub fn new() -> SnmptradpCollector {
        Default::default()
    }
}

impl<'a> Collector<&'a str> for SnmptradpCollector {
    fn to_event(&self, json_trapd_str: &'a str) -> Result<Event, CollectorError> {
        let mut trapd = serde_json::from_str::<Payload>(json_trapd_str)
            .map_err(|e| CollectorError::JsonParsingError { message: format!("{}", e) })?;

        let mut event = Event::new("snmptrapd");

        self.parse_address(&mut trapd, &mut event.payload);

        if let Some(oids) = trapd.remove("VarBinds") {
            event.payload.insert("oids".to_owned(), oids);
        }

        Ok(event)
    }
}

impl SnmptradpCollector {
    fn parse_address(&self, trapd: &mut Payload, event_payload: &mut Payload) {
        if let Some(received_from) = trapd
            .get("PDUInfo")
            .and_then(|value| value.child("receivedfrom"))
            .and_then(|value| value.text())
        {
            match self.address_regex.captures_iter(received_from).next() {
                Some(capture) => {
                    event_payload.insert("protocol".to_owned(), Value::Text(capture[1].to_owned()));
                    event_payload.insert("src_ip".to_owned(), Value::Text(capture[2].to_owned()));
                    event_payload.insert("src_port".to_owned(), Value::Text(capture[3].to_owned()));
                    event_payload.insert("dest_ip".to_owned(), Value::Text(capture[4].to_owned()));
                }
                None => {
                    // Return a result Error
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;

    #[test]
    fn should_produce_event_from_snmptrapd_json_input() {
        // Arrange
        let trapd_filename = "./test_resources/snmptrapd_01_input.json";
        let trapd_string = fs::read_to_string(trapd_filename)
            .expect(&format!("Unable to open the file [{}]", trapd_filename));

        let expected_event_filename = "./test_resources/snmptrapd_01_output.json";
        let expected_event_string = fs::read_to_string(expected_event_filename)
            .expect(&format!("Unable to open the file [{}]", expected_event_filename));
        let mut expected_event =
            serde_json::from_str::<Event>(&expected_event_string).expect("should parse the event");

        let collector = SnmptradpCollector::new();

        // Act
        let event = collector.to_event(&trapd_string).unwrap();

        // Assert
        assert_eq!("snmptrapd", &event.event_type);

        expected_event.created_ts = event.created_ts.clone();

        assert_eq!(expected_event, event);
    }
}
