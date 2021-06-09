use log::trace;
use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::{Event, Payload};

/// A collector that receives an input JSON and unmarshalls/deserializes it directly into an Event struct
#[derive(Default)]
pub struct JsonEventCollector {}

impl JsonEventCollector {
    pub fn new() -> JsonEventCollector {
        Default::default()
    }
}

impl<'a> Collector<&'a str> for JsonEventCollector {
    fn to_event(&self, input: &'a str) -> Result<Event, CollectorError> {
        trace!("JsonEventCollector - received event: {}", input);
        serde_json::from_str::<tornado_common_api::Event>(&input)
            .map_err(|e| CollectorError::EventCreationError { message: format!("{}", e) })
    }
}

/// A collector that receives an input JSON and creates an Event whose payload is the JSON input
pub struct JsonPayloadCollector {
    event_type: String,
}

impl JsonPayloadCollector {
    pub fn new<S: Into<String>>(event_type: S) -> JsonPayloadCollector {
        JsonPayloadCollector { event_type: event_type.into() }
    }
}

impl<'a> Collector<&'a str> for JsonPayloadCollector {
    fn to_event(&self, json_str: &'a str) -> Result<Event, CollectorError> {
        let json = serde_json::from_str::<Payload>(json_str)
            .map_err(|e| CollectorError::JsonParsingError { message: format!("{}", e) })?;
        Ok(Event::new_with_payload(self.event_type.to_owned(), json))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::fs;

    #[test]
    fn should_return_an_event() {
        // Arrange
        let event = Event::new(String::from("email"));
        let json = serde_json::to_string(&event).unwrap();

        let collector = JsonEventCollector::new();

        // Act
        let from_json = collector.to_event(&json).unwrap();

        // Assert
        assert_eq!(event.event_type, from_json.event_type);
        assert_eq!(event.created_ms, from_json.created_ms);
    }

    #[test]
    fn should_return_an_error() {
        // Arrange
        let json = "{message: 'hello_world'}".to_owned();
        let collector = JsonEventCollector::new();

        // Act
        let result = collector.to_event(&json);

        // Assert
        assert!(result.is_err())
    }

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

        let collector = JsonPayloadCollector::new("syslog");

        // Act
        let event = collector.to_event(&rsyslog_string).unwrap();

        // Assert
        assert_eq!("syslog", &event.event_type);
        assert_eq!("2018-11-01T23:59:59+01:00", event.payload.get("@timestamp").unwrap());

        expected_event.trace_id = event.trace_id.clone();
        expected_event.created_ms = event.created_ms.clone();
        assert_eq!(expected_event, event);
    }
}
