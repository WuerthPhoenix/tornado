extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate tornado_collector_common;
extern crate tornado_common_api;

use regex::Captures;
use regex::Regex;
use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::{Event, Payload, Value};

const ADDRESS_PARSE_REGEX: &str = r#"^([^:]+):\s\[([^\]]+)\]:([0-9]+)->\[([^\]]+)\]"#;

/// A collector that receives snmptrapd input messages formatted as JSON, and creates the related Event struct
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

        self.parse_address(&mut trapd, &mut event.payload)?;

        if let Some(oids) = trapd.remove("VarBinds") {
            event.payload.insert("oids".to_owned(), oids);
        }

        Ok(event)
    }
}

impl SnmptradpCollector {
    fn parse_address(
        &self,
        trapd: &mut Payload,
        event_payload: &mut Payload,
    ) -> Result<(), CollectorError> {
        if let Some(received_from) = trapd
            .get("PDUInfo")
            .and_then(|value| value.get_from_map("receivedfrom"))
            .and_then(|value| value.get_text())
        {
            if let Some(capture) = self.address_regex.captures_iter(received_from).next() {
                self.insert_into_payload(&capture, 1, "protocol", event_payload, received_from)?;
                self.insert_into_payload(&capture, 2, "src_ip", event_payload, received_from)?;
                self.insert_into_payload(&capture, 3, "src_port", event_payload, received_from)?;
                self.insert_into_payload(&capture, 4, "dest_ip", event_payload, received_from)?;
            }
        }
        Ok(())
    }

    fn insert_into_payload(
        &self,
        capture: &Captures,
        group: usize,
        field: &str,
        event_payload: &mut Payload,
        input: &str,
    ) -> Result<(), CollectorError> {
        let protocol = capture.get(group).ok_or_else(|| self.error(field, input))?;
        event_payload.insert(field.to_owned(), Value::Text(protocol.as_str().to_owned()));
        Ok(())
    }

    fn error(&self, field: &str, input: &str) -> CollectorError {
        CollectorError::EventCreationError {
            message: format!("Cannot extract [{}] from [{}]", field, input),
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
